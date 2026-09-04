#!/usr/bin/env sh
set -eu

package_dir=${1:?"usage: verify-release-package.sh <package-directory>"}

test -f "$package_dir/simple-lector-dni" || test -f "$package_dir/simple-lector-dni.exe"
test -f "$package_dir/.java-version"
test -f "$package_dir/engine/simple-lector-dni-engine.jar"
test -f "$package_dir/runtime/bin/java" || test -f "$package_dir/runtime/bin/java.exe"
test -f "$package_dir/LICENSE"
test -f "$package_dir/THIRD_PARTY_NOTICES.md"
test -f "$package_dir/THIRD_PARTY_LICENSES.md"
test -f "$package_dir/THIRD_PARTY_LICENSES.html"
test -f "$package_dir/README.md"
test -f "$package_dir/RUNTIME_SOURCE.md"
test -f "$package_dir/runtime/release"
test -f "$package_dir/protocol/engine-v1.schema.json"

expected_java_version=$(tr -d '\r\n' < "$package_dir/.java-version" | sed 's/+.*$//')
rg -Fqx "JAVA_VERSION=\"$expected_java_version\"" "$package_dir/runtime/release"

if test -x "$package_dir/simple-lector-dni"; then
  "$package_dir/simple-lector-dni" --version >/dev/null
  scripts/verify-worker-package.sh \
    "$package_dir/runtime/bin/java" \
    "$package_dir/engine/simple-lector-dni-engine.jar"
fi
