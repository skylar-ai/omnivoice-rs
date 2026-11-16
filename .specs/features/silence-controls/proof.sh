#!/usr/bin/env bash
# Run exactly one cargo test and fail unless exactly one test ran and passed.
# usage: proof.sh <cargo test selector args...> <exact test path>
set -uo pipefail
cd "$(dirname "$0")/../../.."
name="${@: -1}"
args=("${@:1:$#-1}")
out=$(cargo test -q "${args[@]}" -- --exact "$name" --test-threads=1 2>&1)
status=$?
echo "$out" | tail -20
[ $status -eq 0 ] || exit $status
echo "$out" | grep -q "test result: ok. 1 passed" || { echo "proof.sh: expected exactly 1 test named $name"; exit 1; }
