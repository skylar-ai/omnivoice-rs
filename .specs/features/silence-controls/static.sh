#!/usr/bin/env bash
# Static wiring proofs for checks a CPU test cannot reach without model weights.
set -euo pipefail
cd "$(dirname "$0")/../../.."
P=crates/omnivoice-infer/src/pipeline.rs
fn_body() { awk -v f="fn $2(" 'index($0,f){on=1} on{print} on&&/^    }$/{exit}' "$1"; }
case "$1" in
  c5)
    # reference-case and debug decode keep the parity shaping (3-arg decode_final / debug_decode)
    for f in decode_stage1_final_from_reference_case debug_stage1_from_reference_case; do
      body=$(fn_body $P $f)
      echo "$body" | grep -q "generation_config" && { echo "$f reads generation_config"; exit 1; }
      echo "$body" | grep -Eq "\.(decode_final|debug_decode)\(" || { echo "$f no longer calls decode_final/debug_decode"; exit 1; }
      echo "$body" | grep -q "decode_final_with" && { echo "$f uses decode_final_with"; exit 1; }
    done
    grep -q "OutputShaping::default()" crates/omnivoice-infer/src/stage1_decoder.rs || { echo "stage1 does not default to parity shaping"; exit 1; }
    ;;
  c14)
    # callers: both ref_audio prompt builders receive the request's value
    c=$(grep -c "request.generation_config.prompt_trailing_silence," $P)
    [ "$c" -eq 2 ] || { echo "expected 2 callers passing prompt_trailing_silence, found $c"; exit 1; }
    # builders: each forwards its parameter into prepare_prompt_audio_with
    for f in create_voice_clone_prompt_from_audio_with create_device_voice_clone_prompt_from_audio; do
      call=$(fn_body $P $f | awk '/prepare_prompt_audio_with\(/{on=1} on{print} on&&/\)\?;/{exit}')
      [ -n "$call" ] || { echo "$f does not call prepare_prompt_audio_with"; exit 1; }
      echo "$call" | grep -Eq "^\s*prompt_trailing_silence,\s*$" || { echo "$f does not forward prompt_trailing_silence"; exit 1; }
    done
    # the parity wrapper is the only place that passes None
    fn_body $P create_voice_clone_prompt_from_audio | grep -q "None" || { echo "parity wrapper changed"; exit 1; }
    grep -A2 "fn normalize_request_inputs" $P | grep -q "generation_config.validate()" || { echo "validation is not first in normalize_request_inputs"; exit 1; }
    ;;
  live-decode)
    # both live decode sites (short and chunked) pass the request's shaping
    c=$(grep -c "decode_final_with(" $P); [ "$c" -eq 2 ] || { echo "expected 2 live decode_final_with calls, found $c"; exit 1; }
    c=$(grep -c "task.generation_config.output_shaping()," $P); [ "$c" -eq 2 ] || { echo "expected 2 live decodes passing output_shaping(), found $c"; exit 1; }
    ;;
  c21)
    for k in pad_duration fade_duration prompt_trailing_silence; do
      case $k in fade_duration) r='`0`–`1`';; *) r='`0`–`2`';; esac
      case $k in prompt_trailing_silence) d='none';; *) d='`0.1`';; esac
      grep -F "| \`$k\` | $d |" docs/generation-parameters.md >/dev/null || { echo "generation-parameters.md: no default row for $k"; exit 1; }
      grep -F "| \`$k\` |" docs/generation-parameters.md | grep -F "Range $r" >/dev/null || { echo "generation-parameters.md: no range for $k"; exit 1; }
      grep -F "\`$k\` (seconds, default $d, range $r)" docs/cli-and-server.md >/dev/null || { echo "cli-and-server.md: no default/range for $k"; exit 1; }
    done
    ;;
  *) echo "unknown check $1"; exit 2;;
esac
echo "ok $1"
