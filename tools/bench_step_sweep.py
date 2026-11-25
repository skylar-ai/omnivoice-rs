#!/usr/bin/env python3
"""Benchmark omnivoice-cli infer: blog matrix + production-shaped matrix.

Sweeps num_step, guidance_scale, and t_shift, and records wall time, audio
length, RTF, and per-stage timings for each run. See
docs/benchmarks/2026-11-25-step-count-sweep.md.

usage: bench_step_sweep.py OUT_DIR REF_WAV REF_TEXT_FILE [--only blog|prod]

Build the CLI first with `pixi run build-cuda`. Set OMNIVOICE_BENCH_MODEL to a
model folder that contains omnivoice.artifacts.json.
"""
import json, os, statistics, subprocess, sys
from pathlib import Path

CLI = Path(__file__).resolve().parents[1] / "target" / "cuda" / "release" / "omnivoice-cli"
MODEL = os.environ.get("OMNIVOICE_BENCH_MODEL") or sys.exit("set OMNIVOICE_BENCH_MODEL")
OUT = Path(sys.argv[1]); OUT.mkdir(parents=True, exist_ok=True)
REF_AUDIO = sys.argv[2]
REF_TEXT = Path(sys.argv[3]).read_text().strip()
ONLY = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
WARMUP, REPEAT, SEED = 3, 3, 11

TEXTS = {
    "short": "Hello, welcome to the show.",
    "medium": "The quick brown fox jumps over the lazy dog, while the morning sun slowly rises above the quiet hills.",
    "long": ("Text to speech systems have improved dramatically over the last few years. Modern models can clone a "
             "voice from just a few seconds of audio, speak hundreds of languages, and run faster than real time on "
             "a single graphics card. The remaining challenge is to make them cheap and fast enough for production."),
}
T_SHIFTS = [0.1, 0.3]
PROD_FLAGS = ["--instruct", "american accent", "--pad-duration", "0.1", "--fade-duration", "0.02",
              "--prompt-trailing-silence", "0.5"]

# (suite, mode, name, steps, guidance, t_shift, extra_flags)
MATRIX = []
for mode in ["auto", "clone"]:
    for name, st, gs in [("32_2.0", 32, 2.0), ("16_2.0", 16, 2.0), ("10_2.5", 10, 2.5), ("8_2.5", 8, 2.5)]:
        for ts in T_SHIFTS:
            MATRIX.append(("blog", mode, f"{name}_ts{ts}", st, gs, ts, []))
for st in [32, 12, 10, 8]:
    for gs in [3.0, 2.5]:
        if st == 32 and gs == 2.5:
            continue
        for ts in T_SHIFTS:
            MATRIX.append(("prod", "clone", f"{st}_{gs}_ts{ts}", st, gs, ts, PROD_FLAGS))
BASELINE = {"blog": "32_2.0_ts0.1", "prod": "12_3.0_ts0.1"}
if ONLY:
    MATRIX = [m for m in MATRIX if m[0] == ONLY]

rows = []
for suite, mode, cname, steps, gs, ts, extra in MATRIX:
    for tname, text in TEXTS.items():
        tag = f"{suite}_{mode}_{cname}_{tname}"
        bj = OUT / f"{tag}.json"
        cmd = [str(CLI), "infer", "--model", MODEL, "--text", text, "--language", "en",
               "--output", str(OUT / f"{tag}.wav"), "--device", "cuda", "--seed", str(SEED),
               "--num-step", str(steps), "--guidance-scale", str(gs), "--t-shift", str(ts),
               "--warmup", str(WARMUP), "--repeat", str(REPEAT), "--bench-json", str(bj), *extra]
        if mode == "clone":
            cmd += ["--ref-audio", REF_AUDIO, "--ref-text", REF_TEXT]
        p = subprocess.run(cmd, capture_output=True, text=True)
        if p.returncode != 0:
            print(f"FAIL {tag}\n{p.stderr[-2000:]}", flush=True)
            continue
        d = json.loads(bj.read_text())
        its = d["iterations"]
        wall = statistics.median(i["wall_ms"] for i in its) / 1000
        audio = statistics.median(i["duration_s"] for i in its)
        rows.append(dict(suite=suite, mode=mode, config=cname, steps=steps, gs=gs, t_shift=ts, text=tname,
                         wall_s=wall, audio_s=audio, rtf=wall / audio, dtype=d["dtype_resolved"],
                         stages=its[-1]["stage_timings_ms"]))
        print(f"{tag:40s} wall={wall:6.3f}s audio={audio:6.2f}s RTF={wall/audio:.4f}", flush=True)
        (OUT / "summary.json").write_text(json.dumps(rows, indent=2))

for suite, mode in sorted({(r["suite"], r["mode"]) for r in rows}):
    sub = [r for r in rows if r["suite"] == suite and r["mode"] == mode]
    base = {r["text"]: r["rtf"] for r in sub if r["config"] == BASELINE[suite]}
    print(f"\n== {suite} / {mode}  (speedup vs {BASELINE[suite]}) ==")
    print(f"{'config':16s} " + "  ".join(f"{t:>16s}" for t in TEXTS) + "   mean")
    for cname in dict.fromkeys(r["config"] for r in sub):
        cells, sp = [], []
        for t in TEXTS:
            r = next((r for r in sub if r["config"] == cname and r["text"] == t), None)
            if r and t in base:
                s = base[t] / r["rtf"]; sp.append(s); cells.append(f"{r['rtf']:.4f} ({s:.2f}x)")
            else:
                cells.append("-")
        print(f"{cname:16s} " + "  ".join(f"{c:>16s}" for c in cells) + f"   {statistics.mean(sp) if sp else 0:.2f}x")
