# Silence controls verification

**Verdict**: FAIL
**Mutations**: 3 - probed C6, C18, C23 (0 killed, 3 not)
**Proofs**: `proofs.md` at 516f49c
**Verifier**: independent sub-agent (author != verifier)

## Findings

| Finding | Evidence | Fix | Closed |
| --- | --- | --- | --- |
| A live decode can ignore the request's pad/fade and C6 stays green. AC 2/3 then do nothing for that path. | mutation `short live decode ignores request pad/fade (parity default)` not killed: `pipeline.rs:738-743` passed `crate::contracts::OutputShaping::default()` and `static.sh live-decode` passed. The grep only counts `decode_final_with(` >= 2 (`static.sh:29`) and never looks at the argument. | Make the static proof require `task.generation_config.output_shaping()` at both live sites (count == 2 of that argument inside `decode_final_with(` calls), or move the shaping choice into a pure helper a CPU test can call | |
| The device prompt builder can drop `prompt_trailing_silence` and C18 stays green. AC 14 is broken on the device path that `generate` uses. | mutation `device prompt builder drops prompt_trailing_silence` not killed: `pipeline.rs:652` set to `None`, and `static.sh c14` passed. The grep counts the literal `request.generation_config.prompt_trailing_silence,` at the two callers (`static.sh:21`). It never checks that `create_device_voice_clone_prompt_from_audio` forwards its parameter to `prepare_prompt_audio_with`. The same gap exists in `create_voice_clone_prompt_from_audio_with` (`pipeline.rs:204-209`). | Extend `c14` to check that both prompt builders' bodies pass `prompt_trailing_silence` (not `None`) to `prepare_prompt_audio_with`, or add a CPU test at that seam | |
| AC 19 covers `infer` and `infer-batch`, but C23 only exercises `infer`. Removing the `infer-batch` parse-time range check goes unnoticed: the batch command then resolves and loads models before the pipeline rejects the config. | mutation `infer-batch skips parse-time range check` not killed: `main.rs:810` `silence.validate()?;` removed, and `cli_compat out_of_range_fade_exits_before_model_load` passed. The test runs only `infer` (`cli_compat.rs:129-153`). | Loop the C23 test over `infer` and `infer-batch` (with `--test-list`/`--res-dir` and a nonexistent `--model`), or add a unit test that `CliCommand::parse` of `infer-batch ... --fade-duration 1.5` returns `Err` naming `fade_duration` | |
| C24's docs proof (`static.sh c21`) greps only the parameter names. AC 21 requires each doc to list the default and range, and the grep would pass with both missing. Today's docs do carry them (`docs/cli-and-server.md`, `docs/generation-parameters.md`), so no content is wrong now. | `static.sh:33-37` - `grep -q "$k"` per name only | Also grep each doc for the range strings (`0`–`2`, `0`–`1`) and the defaults next to each name | |

## Own examples (all held)

- Trailing silence `s = 2.0` on a 24 240-sample tone (hop remainder 240) + 30 000 zeros, preprocess off and on: zero run 47 760, in [47 041, 48 000]; the sample before the run is above -50 dBFS; length % 960 == 0.
- Inexact floats `s = 0.3, 0.07, 1.37` (preprocess off): runs 6 720 / 960 / 32 640, each within [round(s*sr) - 959, round(s*sr)].
- Fade 1.0 s on a 7 201-sample clip with pad 2.0: the ramp is capped at len/2 = 3 600, sample 3 599 reaches the level, 48 000 zeros each side; pad 0.35 adds exactly floor(8 400).
- Server: JSON `prompt_trailing_silence: 0` / `0.0` gives 200 with `Some(0.0)`. JSON `null` and `"0.5"` give 422 "must be a number". Multipart `""` and `"  0.5"` give 422 "must be a number". Multipart `NaN` and `1e400` give 422 "must be finite". None of these reached the runtime. This matches the plan ("422 when ... not a number"). Note that a client that serializes an unset Option as `null` gets 422, which is the same as the existing float fields.
- Edge (no rule in the plan): 500 loud samples + 1 s zeros, preprocess off. `None` is accepted (24 000 samples). `s = 0` returns "reference audio is too short after hop alignment". `s = 0.5` is accepted with a run of 11 980. Recorded, not a finding.

## Assertions

The criteria-level tests (C2, C3, C7, C8, C10-C17) hold the plan's values. C11's `speech_len` bound is loose, but `assert_tail` also requires the sample before the run to be above -50 dBFS, and that pins the noise as gone. The loose proofs are the three static greps above (C6, C18, C24-docs) and C23's single-subcommand coverage.
