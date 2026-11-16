# Silence controls checks

Mutations: 3
Build: `cargo build -p omnivoice-infer -p omnivoice-cli -p omnivoice-server --tests`; `cargo clippy --workspace --all-targets -- -D warnings`
Plan: `.specs/features/silence-controls/plan.md`

24 checks in 3 slices · 2 one-way doors · 0 open

Proofs run through `proof.sh`, which fails unless exactly one named cargo test ran and passed.
`static.sh` holds the wiring checks that a CPU test can't reach without model weights.

## Checks

### S1 - Output padding and fade

**C1** - `GenerationConfig::default()` has `pad_duration == 0.1`, `fade_duration == 0.1`, `prompt_trailing_silence == None` (AC 1)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls generation_config_defaults`

**C2** - Output shaping with `pad_duration = 0.25` at 24 000 Hz adds exactly 6 000 zero samples on each side; `0` adds none (AC 2)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls output_pad_adds_exact_zero_samples`

**C3** - Output shaping with `fade_duration = 0.02` at 24 000 Hz on a 1 s constant clip ramps over 480 samples at each end (linspace with endpoints, as upstream: samples 0..479 and the last 479 below the constant, the 480th at it) and leaves 480..len-480 unchanged; `0` leaves every speech sample unchanged (AC 3)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls output_fade_ramps_exact_sample_count`

**C4** - With the default config, output shaping equals the previous hard-coded path (`remove_silence(500,100,100)` when post-processing is on, RMS restore or peak normalize, `fade_and_pad_audio(0.1, 0.1)`) sample for sample, across post-processing on/off × `ref_rms` Some/None (AC 4)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls default_output_shaping_matches_legacy_path`

**C5** - The reference-case and debug decode paths keep the parity shaping and never read the request config (AC 5)
Proof: `.specs/features/silence-controls/static.sh c5`

**C6** - Both live decode sites (short and chunked) in the pipeline pass the request's output shaping (AC 2, AC 3)
Proof: `.specs/features/silence-controls/static.sh live-decode`

**C7** - `GenerationConfig::validate` rejects `pad_duration` NaN, -0.01 and 2.0001 with `InvalidRequest` naming `pad_duration`, and accepts 0 and 2.0 (AC 6)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls pad_duration_bounds`

**C8** - `GenerationConfig::validate` rejects `fade_duration` infinity, -0.01 and 1.01 with `InvalidRequest` naming `fade_duration`, and accepts 0 and 1.0 (AC 7)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls fade_duration_bounds`

**C9** - The pipeline rejects an invalid config before it prepares any prompt or touches a model (AC 6, AC 7, AC 13)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --lib pipeline::tests::invalid_silence_controls_rejected_before_model_work`

### S2 - Prompt trailing silence

**C10** - With preprocessing on and `s = 0.5` (sr 24 000, hop 960), a 1 s tone + 1 s zeros prepares to the same tone followed by a zero run of length in [11 041, 12 000] samples, with the sample before the run above -50 dBFS (AC 8)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_with_preprocess_zero_tail`

**C11** - With preprocessing on and `s = 0.5`, a 1 s tone + 1 s of -60 dBFS noise loses the noise and ends in the same zero run (AC 8)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_with_preprocess_strips_noise`

**C12** - With preprocessing off and `s = 0.5`, a 1 s tone + 0.1 s zeros keeps the tone samples identical to the `None` result and ends in a zero run in [11 041, 12 000] (AC 9)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_without_preprocess`

**C13** - With `None`, the prepared waveform, `ref_rms` and `ref_text` equal the existing `prepare_prompt_audio` result, with preprocessing on and off (AC 10)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_none_is_unchanged`

**C14** - With `s = 0.5`, `ref_rms` and `ref_text` equal the values prepared with `None` for the same input, with preprocessing on and off (AC 11)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_keeps_rms_and_text`

**C15** - With preprocessing off, an all-zero reference with `s = 0.5` returns `InvalidRequest` containing "empty after silence removal"; the same input with `None` is accepted (AC 12)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls silent_reference_with_trailing_silence_rejected`

**C16** - `GenerationConfig::validate` rejects `prompt_trailing_silence = Some(2.5)`, `Some(-0.1)` and `Some(NaN)` naming `prompt_trailing_silence`, and accepts `Some(0.0)`, `Some(2.0)` and `None` (AC 13)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls prompt_trailing_silence_bounds`

**C17** - `s = 0` strips trailing near-silence and appends no zeros: a 1 s tone + 1 s zeros prepares to a waveform whose last sample is above -50 dBFS or within one hop of it (AC 13)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_zero_strips_only`

**C18** - When a request carries `ref_audio`, the pipeline builds its prompt with the request's `prompt_trailing_silence` (AC 14)
Proof: `.specs/features/silence-controls/static.sh c14`

### S3 - CLI and server

**C19** - A JSON speech request with `pad_duration 0.05`, `fade_duration 0.02` and `prompt_trailing_silence 0.5` reaches the runtime with those three values (AC 15)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server json_speech_forwards_silence_controls`

**C20** - A multipart speech request with the same three text fields reaches the runtime with the same values; `fade_duration=abc` answers 422 `invalid_request_error` with "fade_duration must be a number" (AC 16)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server multipart_speech_forwards_silence_controls`
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server multipart_non_numeric_fade_is_422`

**C21** - A speech request with `pad_duration: 3` answers 422 `invalid_request_error` naming `pad_duration`, and the runtime records no call (AC 17)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server out_of_range_pad_is_422_without_runtime_call`

**C22** - `infer` and `infer-batch` parse `--pad-duration 0.05 --fade-duration 0.02 --prompt-trailing-silence 0.5` and the `_` aliases into those config values (AC 18)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-cli --bin omnivoice-cli tests::infer_parses_silence_controls`
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-cli --bin omnivoice-cli tests::infer_batch_parses_silence_controls`

**C23** - `infer` and `infer-batch` with `--fade-duration 1.5` and a nonexistent `--model` exit non-zero and stderr names `fade-duration`/`fade_duration`, not the missing model or test list (AC 19)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-cli --test cli_compat out_of_range_fade_exits_before_model_load`

**C24** - `infer --help` and `infer-batch --help` list `--pad-duration`, `--fade-duration` and `--prompt-trailing-silence` with defaults `0.1`, `0.1` and `none`; the docs list all three parameters (AC 20, AC 21)
Proof: `.specs/features/silence-controls/proof.sh -p omnivoice-cli --test cli_compat help_lists_silence_controls`
Proof: `.specs/features/silence-controls/static.sh c21`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| new config fields (3) | `pad_duration` C1 C2 C7 · `fade_duration` C1 C3 C8 · `prompt_trailing_silence` C1 C10 C16 | - |
| pad examples, AC 2 (2) | 0.25 C2 · 0 C2 | - |
| fade examples, AC 3 (2) | 0.02 C3 · 0 C3 | - |
| default parity, AC 4 (6) | post on + rms Some C4 · post on + rms None C4 · post off + rms Some C4 · post off + rms None C4 · single decode C6 · chunked decode C6 | - |
| pad bounds, AC 6 (4) | 2.0 C7 · 2.0001 C7 · -0.01 C7 · NaN C7 | - |
| fade bounds, AC 7 (3) | 1.0 C8 · 1.01 C8 · infinity C8 | - |
| trailing examples, AC 8 (2) | zeros tail C10 · -60 dBFS noise tail C11 | - |
| trailing preprocess modes (2) | on C10 C11 · off C12 | - |
| trailing bounds, AC 13 (3) | 0 C16 C17 · 2.5 C16 · negative/NaN C16 | - |
| silent reference, AC 12 (2) | set → error C15 · None → accepted C15 | - |
| `POST /v1/audio/speech` statuses this changes (2) | 200 C19 C20 · 422 C20 C21 | - |
| request encodings (2) | JSON C19 · multipart C20 | - |
| CLI subcommands (2) | `infer` C22 C23 C24 · `infer-batch` C22 C23 C24 | - |
| CLI exit codes (2) | 0 C22 · non-zero C23 | - |
| validation entry points (3 assemblies) | pipeline C9 · server C21 · CLI C23 | - |
| Landing doors (2) | door 1 pub fields + version bump C1 · door 2 wire names C19 C20 | - |

- 401 on the speech route is unchanged and stays covered by the existing `openai_server` auth tests.
- `proof.sh` selects one test with `--exact` and fails unless exactly one ran, so the validator's selector warnings don't apply.
- Claims naming a status code or wire field: C19, C20, C21. Each has a proof that crosses the HTTP router.

## Swept

- validation: C7, C8, C9, C16, C20, C21, C23
- failure modes: C15
- idempotency: n/a - generation is a pure function of request and seed; nothing is persisted
- authorization: existing - the bearer-key middleware covers the speech route; no new route
- concurrency: n/a - the new values ride in the per-request `GenerationConfig`, with no shared state
- data lifecycle: n/a - nothing stored; artifact JSON is untouched (C5)
- dependency failure: n/a - no new external call
- state transitions: n/a - no stateful entity
- observability: n/a - no logging requirement; errors name the field (C7, C8, C16)

## Handoff

- **Settled mid-build:** none
- **Dispatched:** Verifier at `516f49c` (no screens: no capture binds) - returned
- **Boundary:** C1-C17 closed at e537fd4 (infer); C18-C24 closed in the cli/server commit
- **Abandoned:** none
- **Corrected:** C3's wording before its test went green: the fade is numpy-style linspace with endpoints (upstream behaviour), so the 480th ramp sample is at full level; the original wording claimed all 480 were below it. AC 3's example in plan.md was corrected the same way.
- **Proof scripts:** `static.sh c14` was tightened during the build to require both prompt builders (host and device paths) and validation first in `normalize_request_inputs`.
- **Pre-existing failure:** `_bench_scenarios::bench_listening_scenarios` panics without `OMNIVOICE_BENCH_CONFIG`; unrelated and unchanged.
