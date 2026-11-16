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
    # both ref_audio prompt builders (host materialize_request, device materialize_device_request)
    c=$(grep -c "request.generation_config.prompt_trailing_silence," $P)
    [ "$c" -eq 2 ] || { echo "expected 2 prompt builders passing prompt_trailing_silence, found $c"; exit 1; }
    grep -q "fn normalize_request_inputs" $P && grep -A2 "fn normalize_request_inputs" $P | grep -q "generation_config.validate()" || { echo "validation is not first in normalize_request_inputs"; exit 1; }
    ;;
  live-decode)
    # both live decode sites (short and chunked) pass the request's shaping
    c=$(grep -c "decode_final_with(" $P); [ "$c" -ge 2 ] || { echo "live decodes do not pass output shaping ($c)"; exit 1; }
    grep -Eq "decode_final\(\s*$" $P && { echo "a live decode still uses the parity decode_final"; exit 1; }
    ;;
  c21)
    for d in docs/generation-parameters.md docs/cli-and-server.md; do
      for k in pad_duration fade_duration prompt_trailing_silence; do
        grep -q "$k" "$d" || { echo "$d misses $k"; exit 1; }
      done
    done
    ;;
  *) echo "unknown check $1"; exit 2;;
esac
echo "ok $1"
