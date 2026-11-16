# silence-controls proofs

**HEAD**: f4826c1
**Tree**: clean
**Result**: 29 pass, 0 fail; mutations 3 killed, 0 not

| Check | Proof | Ran | Passed | Result |
| --- | --- | --- | --- | --- |
| C1 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls generation_config_default` | 1 | - | pass |
| C2 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls output_pad_adds_exact_zer` | 1 | - | pass |
| C3 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls output_fade_ramps_exact_s` | 1 | - | pass |
| C4 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls default_output_shaping_ma` | 1 | - | pass |
| C5 | `.specs/features/silence-controls/static.sh c5` | count unknown | - | pass |
| C6 | `.specs/features/silence-controls/static.sh live-decode` | count unknown | - | pass |
| C7 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls pad_duration_bounds` | 1 | - | pass |
| C8 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls fade_duration_bounds` | 1 | - | pass |
| C9 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --lib pipeline::tests::invalid_silence_controls_r` | 1 | - | pass |
| C10 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_with_pre` | 1 | - | pass |
| C11 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_with_pre` | 1 | - | pass |
| C12 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_without_` | 1 | - | pass |
| C13 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_none_is_` | 1 | - | pass |
| C14 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_keeps_rm` | 1 | - | pass |
| C15 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls silent_reference_with_tra` | 1 | - | pass |
| C16 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls prompt_trailing_silence_b` | 1 | - | pass |
| C17 | `.specs/features/silence-controls/proof.sh -p omnivoice-infer --test silence_controls trailing_silence_zero_str` | 1 | - | pass |
| C18 | `.specs/features/silence-controls/static.sh c14` | count unknown | - | pass |
| C19 | `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server json_speech_forwards_silenc` | 1 | - | pass |
| C20 | `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server multipart_speech_forwards_s` | 1 | - | pass |
| C20 | `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server multipart_non_numeric_fade_` | 1 | - | pass |
| C21 | `.specs/features/silence-controls/proof.sh -p omnivoice-server --test openai_server out_of_range_pad_is_422_wit` | 1 | - | pass |
| C22 | `.specs/features/silence-controls/proof.sh -p omnivoice-cli --bin omnivoice-cli tests::infer_parses_silence_con` | 1 | - | pass |
| C22 | `.specs/features/silence-controls/proof.sh -p omnivoice-cli --bin omnivoice-cli tests::infer_batch_parses_silen` | 1 | - | pass |
| C23 | `.specs/features/silence-controls/proof.sh -p omnivoice-cli --test cli_compat out_of_range_fade_exits_before_mo` | 1 | - | pass |
| C24 | `.specs/features/silence-controls/proof.sh -p omnivoice-cli --test cli_compat help_lists_silence_controls` | 1 | - | pass |
| C24 | `.specs/features/silence-controls/static.sh c21` | count unknown | - | pass |
| Build | `cargo build -p omnivoice-infer -p omnivoice-cli -p omnivoice-server --tests` | - | - | pass |
| Build | `cargo clippy --workspace --all-targets -- -D warnings` | - | - | pass |

## Mutations

| Mutation | Location | Killed |
| --- | --- | --- |
| short live decode ignores request pad/fade (parity default) | `crates/omnivoice-infer/src/pipeline.rs:738` | yes |
| device prompt builder drops prompt_trailing_silence | `crates/omnivoice-infer/src/pipeline.rs:652` | yes |
| infer-batch skips parse-time range check | `crates/omnivoice-cli/src/main.rs:810` | yes |
