#!/usr/bin/env sh
set -eu

if rg -n 'getDg2\(|getDg7\(' engine/jmulticard-worker/src/main; then
  echo "The worker must not read DG2 or DG7" >&2
  exit 1
fi
