# Step count and guidance sweep (November 25, 2026)

This report measures how `num_step`, `guidance_scale`, and `t_shift` affect
inference latency in `omnivoice-cli` on CUDA. It checks how much of the
speedup in the HeyNEO post
[Training-free OmniVoice inference acceleration](https://heyneo.com/blog/omnivoice-inference-acceleration)
carries over to this Rust port, and to the settings that production dubbing
uses.

## Summary

- Cutting steps gives this port about the same gain the blog measured in
  eager PyTorch. With the default voice, 16 steps is 1.8x faster than 32
  steps, and 8 steps is 3.1x faster.
- Voice cloning shrinks the gain to 1.65x (16 steps) and 2.46x (8 steps),
  because two costs don't depend on step count.
- The deployed dubbing service, `skylar-inference-dubs`, runs 32 steps
  (`OMNIVOICE_NUM_STEP=32`), not the `skylar-inference` code default of 12.
  With production request settings, 12 steps is 1.9x faster than 32.
- Against 12 steps with guidance 3.0, 10 steps is 1.11x faster and 8 steps is
  about 1.25x faster.
- Every cloned request spends about 500 ms preparing and encoding the
  reference clip. That's about 40% of a short request at 12 steps, and more
  than you save by going from 12 to 8 steps.
- `guidance_scale` (2.5 or 3.0) and `t_shift` (0.1 or 0.3) don't change
  latency. Times stay within 1%, so choose them on quality alone.
- In a listening test, 12 steps with guidance 3.0 and `t_shift` 0.1 sounded
  best. It beat 32 steps, which added a glitch and pauses. Every config with
  fewer than 12 steps had a quality issue, and `t_shift` 0.3 produced the
  worst file of the run.

- Production logs show that a 32-step call takes about 0.85 s on the
  production GPU with an image built for it, but reached a 25 s median under
  load. The queue depth of 8 let groups wait behind up to 8 others instead of
  being refused. See [Production log metrics](#production-log-metrics).

We recommend these changes:

1. Set `skylar-inference-dubs` to 12 steps. This is the largest speedup
   available, about 1.9x, and the listening test preferred its output.
2. Lower `OMNIVOICE_MAX_QUEUE_DEPTH` to fit a latency budget, so a clip that
   would arrive too late is refused instead of delayed.
3. Cache the encoded reference. On the L4 this removes about 500 ms from each
   request, more than going below 12 steps saves, and it doesn't change the
   audio.

## What the blog changed

The blog reports a 6.17x speedup on an RTX A6000 at batch size 1, compared to
32 steps with guidance 2.0 in eager PyTorch. It combines three changes:

1. `num_step` 32 to 10.
2. `guidance_scale` 2.0 to 2.5.
3. `torch.compile(mode="reduce-overhead")`, which adds CUDA graphs.

Step count accounts for about 1.9x (16 steps, eager) and compilation for
about 2.3x on top of that. Candle runs each operation as its own kernel
launch, so this port has no equivalent of the compilation step. This sweep
covers only the first two changes.

This port already applies several optimizations the blog either relied on or
couldn't get working:

- The conditional and unconditional guidance rows run as one batched forward
  pass (`crates/omnivoice-infer/src/stage0_model.rs`).
- Text embeddings are computed once per request, not once per step.
- Flash Attention v2 runs per row
  (`crates/omnivoice-infer/src/stage0_qwen3.rs`). The blog couldn't use it in
  PyTorch.

## Setup

| Item | Value |
| --- | --- |
| GPU | NVIDIA L4, 23 GB, driver 610.43.02 |
| Commit | `7da2620` (`omnivoice-infer` 0.1.0-skylar-3) |
| Build | `pixi run build-cuda` (release, `cuda` feature, flash-attn) |
| Model | Production bundle from `skylar-inference/.models/omnivoice` |
| Runtime dtype | f16 |
| Batch size | 1 |
| Seed | 11 |
| Runs per config | 3 warm-up runs (discarded), then the median of 3 timed runs |
| Script | [`tools/bench_step_sweep.py`](../../tools/bench_step_sweep.py) |

Every config generates three English texts:

| Name | Text | Audio length |
| --- | --- | --- |
| short | "Hello, welcome to the show." | about 2 s |
| medium | One sentence, 19 words | about 6 to 8 s |
| long | Three sentences, 53 words | about 19 to 22 s |

For voice cloning, the reference is a 10.15 s Brazilian Portuguese clip
(48 kHz, stereo) with its exact transcript. This matches the dubbing use case,
where a Portuguese speaker's voice speaks English. Production caps the
reference at 5.5 s, so the cloning numbers here overstate the per-step cost
production sees.

The benchmark measures wall time for one `pipeline.generate` call, read from
the CLI's `--bench-json` output. Model loading isn't included. The tables
compare wall time rather than real-time factor (RTF), because audio length
changes slightly between configs and seeds, which makes RTF noisy.

## Production settings

The dubbing service is `skylar-inference-dubs` (Cloud Run, `us-central1`).
Its last revision to serve traffic, `skylar-inference-dubs-00007-4qd`, ran an
untagged image built on November 24, 2026 at 19:59 UTC, after the
`v1.7.0-sm120` release commit. The service template now points at the image
tagged `v1.7.0-sm120`. That release uses `skylar-tts-rs` 1.2.0-sm120, which
uses the same `omnivoice-infer` version as this benchmark.

Since November 24, 2026 at 22:54 UTC, the service is set to manual scaling
with 0 instances, so it serves no traffic.

| Parameter | Deployed value | Source |
| --- | --- | --- |
| `num_step` | 32 | Service env `OMNIVOICE_NUM_STEP=32`. The code default is 12, and the `skylar-tts-rs` default is 16. |
| `guidance_scale` | 3.0 for English, 2.0 for other languages | `profile_for()` in `skylar-inference` |
| `instruct` | "american accent" for English only | `profile_for()` in `skylar-inference` |
| `t_shift` | 0.1 | `omnivoice-infer` default |
| `position_temperature` / `class_temperature` | 5.0 / 0.0 | `omnivoice-infer` default |
| `layer_penalty_factor` | 5.0 | `omnivoice-infer` default |
| `pad_duration` / `fade_duration` | 0.1 s / 0.02 s | `skylar-inference` constants |
| `prompt_trailing_silence` | 0.5 s | `skylar-inference` constant |
| Number spelling (`OMNIVOICE_PREPROCESS_TEXT`) | On | `skylar-inference` default |
| Queue depth / texts per call | 8 / 4 | Service env `OMNIVOICE_MAX_QUEUE_DEPTH=8` / `skylar-tts-rs` default |
| dtype | f16 | `skylar-tts-rs` |
| Hardware | 1 NVIDIA RTX PRO 6000, 20 vCPU, 80 GiB, 0 to 3 instances | Cloud Run |

Every revision since the service was created on November 19, 2026 sets
`OMNIVOICE_NUM_STEP=32`. The reference clip length is set by the caller,
`ll-tts`, which caps it at 5.5 s by default.

## Production log metrics

The `skylar-inference-dubs` app logs cover November 19 to 24, 2026: 9,005 TTS
calls from 53 sessions, all at 32 steps with a queue depth of 8. Each call's
`elapsed_ms` covers the whole server side: queue wait, reference encoding,
and generation. Almost every call (99.9%) carried 2 texts, with a median of
4.2 s of audio per call.

| Revision | Dates (UTC) | Instances | Calls/min | Median | p95 | Refused |
| --- | --- | --- | --- | --- | --- | --- |
| 00001 | Nov 19, 14:16 to 14:31 | 1 | 21 | 3.05 s | 6.25 s | 0 |
| 00004 | Nov 19, 16:56 to 18:40 | 3 | 20 | 25.2 s | 27.9 s | 35 |
| 00005 | Nov 19, evening | 1 | 55 | 4.66 s | 7.37 s | 18 |
| 00005 | Nov 24 | 1 | about 20 | 0.89 s | 1.03 s | 0 |
| 00006 | Nov 24, 19:17 to 19:40 | 1 | 20 | 2.70 s | 3.27 s | 0 |
| 00007 | Nov 24, 20:12 to 22:46 | 1 | 22 | 0.85 s | 0.98 s | 0 |

Calls/min is the mean over minutes with traffic. All 53 refusals happened on
November 19 between 17:22 and 18:40 UTC. Revision 00005's 18 refusals came at
18:40, while 00004 was still draining.

### A call takes about 0.85 s on the production GPU

Under light load, revision 00007 served a call in 0.85 s (median), with a p95
of 0.98 s. That's an RTF of about 0.2, including reference encoding. Based on
the step costs measured on the L4, 12 steps would bring this to about 0.45 to
0.5 s. This estimate hasn't been measured on the production GPU.

### The image changes speed by about 3x

At the same load, revision 00006 took 2.70 s per call and revision 00007 took
0.85 s. The p95 of 00006 is close to its median, so the extra time is slower
generation, not queueing. The 00006 image was built at 18:02 UTC, right after
the `v1.7.0` release commit. The 00007 image was built at 19:59 UTC, after the
`v1.7.0-sm120` commit at 19:43. The likely cause is that 00006 ran a build not
compiled for the RTX PRO 6000 (sm120). This hasn't been verified. Revisions
00001 and 00004 show similar slow times.

### A deep queue delays clips instead of refusing them

With 3 instances at about 20 calls/min, revision 00004 reached a 25 s median.
That matches a full queue: 8 groups waiting at about 3 s each. When the queue
was full, the server refused 35 groups, and `ll-tts` dropped them without
retrying. A dubbed clip that arrives 25 s late is likely no longer useful.
With an sm120 image, a full queue of 8 waits about 7 s at 32 steps, or about
4 s at 12 steps.

Revision 00004 saturated with 3 instances at a load that one instance of
00007 handled easily. gRPC streams stay on the instance that opened them,
session affinity is off, and `containerConcurrency` is 1000, so streams can
pile onto one instance while others stay idle. Per-instance data is needed to
confirm this.

### References run slightly over the limit

The server warned about 1,401 references outside the recommended 3 to 5.5 s:
1,346 too long and 55 too short. The long ones have a median of 5.6 s. That's
probably the `ll-tts` cap of 5.5 s plus its 0.15 s lead padding. The warning
doesn't change the output.

## Results

### Blog matrix, default voice

No reference clip. The speedup is the mean across the three texts, relative
to 32 steps with guidance 2.0.

| Steps / guidance | short | medium | long | Speedup |
| --- | --- | --- | --- | --- |
| 32 / 2.0 | 0.59 s | 1.08 s | 3.44 s | 1.00x |
| 16 / 2.0 | 0.31 s | 0.60 s | 1.90 s | 1.84x |
| 10 / 2.5 | 0.21 s | 0.42 s | 1.35 s | 2.63x |
| 8 / 2.5 | 0.17 s | 0.36 s | 1.15 s | 3.14x |

The 16-step result (1.84x) matches the blog's eager 16-step result (1.92x).

### Blog matrix, voice cloning

| Steps / guidance | short | medium | long | Speedup |
| --- | --- | --- | --- | --- |
| 32 / 2.0 | 2.28 s | 3.42 s | 4.82 s | 1.00x |
| 16 / 2.0 | 1.40 s | 2.04 s | 2.92 s | 1.65x |
| 10 / 2.5 | 1.07 s | 1.51 s | 2.18 s | 2.20x |
| 8 / 2.5 | 0.96 s | 1.34 s | 1.95 s | 2.46x |

### Production matrix

This matrix copies the production dubbing request: voice cloning, English with
`instruct` set to "american accent", `pad_duration` 0.1 s, `fade_duration`
0.02 s, and `prompt_trailing_silence` 0.5 s. The deployed service runs 32
steps (see [Production settings](#production-settings)). The speedup is
relative to 12 steps with guidance 3.0, the `skylar-inference` code
default.

| Steps / guidance | short | medium | long | Speedup |
| --- | --- | --- | --- | --- |
| 32 / 3.0 (deployed) | 2.27 s | 3.47 s | 4.97 s | 0.52x |
| **12 / 3.0 (code default)** | **1.23 s** | **1.76 s** | **2.55 s** | **1.00x** |
| 12 / 2.5 | 1.21 s | 1.77 s | 2.55 s | 1.00x |
| 10 / 3.0 | 1.11 s | 1.60 s | 2.29 s | 1.11x |
| 10 / 2.5 | 1.11 s | 1.58 s | 2.29 s | 1.11x |
| 8 / 3.0 | 1.00 s | 1.39 s | 2.04 s | 1.25x |
| 8 / 2.5 | 1.00 s | 1.39 s | 2.04 s | 1.24x |

All rows use `t_shift` 0.1. The `t_shift` 0.3 runs were within 1% of these
times. The raw data for every run is in
[`data/2026-11-25/`](./data/2026-11-25/).

### Where the time goes

The pipeline records time spent in each stage. At 12 steps with guidance 3.0:

| Stage | short | medium | long |
| --- | --- | --- | --- |
| `stage_materialize` (reference clip) | 508 ms | 501 ms | 506 ms |
| `stage0_generate` (12 unmasking steps) | 651 ms | 1060 ms | 1467 ms |
| `stage1_decode` (tokens to waveform) | 44 ms | 163 ms | 521 ms |

From the 32-step and 12-step runs, one unmasking step costs:

| Text | Default voice | Voice cloning |
| --- | --- | --- |
| short | 18 ms | 53 ms |
| long | 96 ms | 122 ms |

`stage0_generate` is almost entirely per-step work. The fixed cost of a
cloned request is `stage_materialize` plus `stage1_decode`.

## Listening results

One listener compared ten voice cloning outputs and ranked them from best to
worst. Each file comes from the production matrix unless the table says
otherwise.

| Rank | Steps / guidance / `t_shift` | Text | Notes |
| --- | --- | --- | --- |
| 1 | 12 / 3.0 / 0.1 (code default) | long | No issues noticed. Good quality. |
| 2 | 32 / 3.0 / 0.1 (deployed) | long | Better voice similarity than rank 1, but a short glitch at the start and some pauses. |
| 3 | 12 / 3.0 / 0.1 (code default) | short | No issues. Good quality and voice similarity. |
| 4 | 10 / 2.5 / 0.1 | long | Similar to rank 5, with the same long pauses, but no repeated words. |
| 5 | 10 / 3.0 / 0.1 | long | Pauses like rank 2. Repeats the word "modern" at 0:07. |
| 6 | 8 / 3.0 / 0.1 | long | Unnatural accent and intonation. Glitches before some words. |
| 7 | 8 / 2.5 / 0.1, blog matrix | long | Understandable, and the voice is cloned, but it sounds robotic. |
| 8 | 8 / 3.0 / 0.1 | short | All words correct, but poor voice similarity. |
| 9 | 10 / 2.5 / 0.3 | short | All words correct, but worse voice similarity. Shortest output of the run (1.64 s). |
| 10 | 8 / 3.0 / 0.3 | long | Wrong words, glitches, and unintelligible passages. Unusable. |

The blog-matrix file used guidance 2.5 without the production accent
instruction or silence settings.

The ranking leads to these conclusions:

- **Use 12 steps.** It produced the only files with no issues on both long
  and short text. More steps didn't help: 32 steps, which production runs
  today, added a glitch and pauses.
- **Fewer steps hurts quality.** At 10 steps, guidance 3.0 repeated a word.
  At 8 steps, long text gained accent and glitches, and short text lost the
  speaker's voice. The speedups (1.11x and 1.25x) don't justify these issues.
- **Short lines lose voice similarity first.** Ranks 8 and 9 kept every word
  but no longer sounded like the speaker. Dubbing is mostly short clauses, so
  test short lines when you evaluate any setting change.
- **Guidance 2.5 did better than 3.0 at 10 steps.** Rank 4 avoided the word
  repeat in rank 5. This comes from one seed, so confirm it on more seeds
  before you rely on it.
- **Reject `t_shift` 0.3.** It produced the worst file of the run and the
  shortest output with the weakest voice similarity.

## Analysis

### Reference encoding is the largest fixed cost

`materialize_device_request` in `crates/omnivoice-infer/src/pipeline.rs`
prepares the reference audio and encodes it with the audio tokenizer on every
request. It takes about 500 ms whether the request uses 8 steps or 32, and
whether the text is short or long. Production sends the same speaker's
reference with many requests, so the server repeats this work each time.

The library already accepts a pre-encoded reference through
`voice_clone_prompts`. Caching the encoded reference per speaker would remove
this cost from every request after the first. That would save more than
going from 12 to 8 steps does, and it doesn't touch output quality. This
benchmark used a 10 s reference, so measure the cost with production's 5.5 s
cap before you size the gain.

### The reference also makes every step more expensive

The reference tokens go into the sequence the model processes at each step.
For short text, one step costs 18 ms with the default voice and 53 ms with
the 10 s reference. A shorter reference cap reduces this cost on every step.

### Guidance and `t_shift` only affect quality

Changing `guidance_scale` between 2.5 and 3.0, or `t_shift` between 0.1 and
0.3, changed wall time by less than 1%. They don't change the amount of work
per step, so pick their values from listening tests.

### CUDA graphs come after the fixed costs

`torch.compile` produced about 2.3x of the blog's gain by cutting kernel
launch overhead. Candle has no built-in equivalent. Capturing one unmasking
step as a CUDA graph is possible, because tensor shapes don't change between
steps, but it needs fixed input buffers and an allocator that is safe to use
during capture. Remove the reference encoding cost first, then profile one
step with `nsys` to see if launch gaps are large enough to justify CUDA
graphs.

## Limits of this benchmark

- **No automated quality scores.** Whisper wasn't available on the benchmark
  machine, so the run has no WER or speaker similarity scores. The listening
  results come from one listener and ten files.
- **One reference clip and three texts.** The results come from one speaker,
  one seed, and English output only.
- **Reference longer than production.** The 10.15 s reference is above the
  production cap of 5.5 s.
- **No target duration.** Production sends a target `duration` for each clip.
  This benchmark doesn't set one.
- **Different GPU.** The blog used an RTX A6000, and production uses an RTX
  PRO 6000. This run used an L4, so compare speedup ratios, not absolute
  times.
- **Logs only.** The production metrics come from app logs. They don't
  include Cloud Monitoring data such as GPU utilization or instance count over
  time.

## Next steps

1. Set `OMNIVOICE_NUM_STEP` to 12 on `skylar-inference-dubs`, or remove it to
   use the code default. Keep guidance 3.0 and `t_shift` 0.1. Find out first
   why the service was set to 32.
2. Set `OMNIVOICE_MAX_QUEUE_DEPTH` from a latency budget: divide the longest
   useful wait by the time per call. At about 0.85 s per call, a depth of 8
   means waits up to about 7 s.
3. Deploy only images built for the production GPU (the `-sm120` builds), and
   confirm that revisions 00001, 00004, and 00006 ran other builds.
4. Check how streams spread across instances. Pull per-instance logs or Cloud
   Monitoring metrics for revision 00004.
5. Add a per-speaker cache of encoded references. `skylar-tts-rs` encodes the
   reference once per call in `generate()` (`src/omnivoice.rs`), and shares
   it across the texts in that call, but keeps nothing between calls.
6. Measure `stage_materialize` with a 5.5 s reference on the production GPU.
7. Optional: If you reconsider fewer steps, test 10 steps with guidance 2.5
   on several seeds and speakers, and include WER and speaker similarity
   scores.

## Reproduce this run

Before you start, build the CLI and set the model folder:

```bash
pixi run build-cuda
export OMNIVOICE_BENCH_MODEL=/path/to/omnivoice-model
```

The model folder must contain `omnivoice.artifacts.json`. The raw Hugging Face
snapshot doesn't, and the CLI fails with `MissingArtifact`.

To run both matrices, follow these steps:

1. Save a reference clip as `ref.wav` and its exact transcript as `ref.txt`.
2. Run the production matrix:

   ```bash
   python3 tools/bench_step_sweep.py out/prod ref.wav ref.txt --only prod
   ```

3. Run the blog matrix:

   ```bash
   python3 tools/bench_step_sweep.py out/blog ref.wav ref.txt --only blog
   ```

Each run writes a WAV file and a `--bench-json` file. When a matrix finishes,
the script writes `summary.json` and prints the speedup tables. On the L4,
the production matrix takes about 11 minutes and the blog matrix about
8 minutes.
