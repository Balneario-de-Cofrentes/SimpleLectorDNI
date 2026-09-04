#!/usr/bin/env sh
set -eu

package_dir=${1:?"usage: verify-release-package.sh <package-directory>"}

test -f "$package_dir/simple-lector-dni" || test -f "$package_dir/simple-lector-dni.exe"
test -f "$package_dir/engine/simple-lector-dni-engine.jar"
test -f "$package_dir/runtime/bin/java" || test -f "$package_dir/runtime/bin/java.exe"
test -f "$package_dir/LICENSE"
test -f "$package_dir/THIRD_PARTY_NOTICES.md"
test -f "$package_dir/protocol/engine-v1.schema.json"

if test -x "$package_dir/simple-lector-dni"; then
  "$package_dir/simple-lector-dni" --version >/dev/null
  scripts/verify-worker-package.sh \
    "$package_dir/runtime/bin/java" \
    "$package_dir/engine/simple-lector-dni-engine.jar"
fi
