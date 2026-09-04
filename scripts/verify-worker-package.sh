#!/usr/bin/env sh
set -eu

java_binary=${1:?"usage: verify-worker-package.sh <java> <worker-jar>"}
worker_jar=${2:?"usage: verify-worker-package.sh <java> <worker-jar>"}

response=$(
  printf '%s\n' '{"protocol":99,"command":"read","reader_index":0}' |
    "$java_binary" -jar "$worker_jar"
)

printf '%s' "$response" | jq -e '
  .protocol == 1 and
  .status == "error" and
  .error.code == "INVALID_REQUEST" and
  .error.retryable == false
' >/dev/null
