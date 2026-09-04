#!/usr/bin/env sh
set -eu

package_dir=${1:?"usage: verify-release-package.sh <package-directory>"}

test -f "$package_dir/simple-lector-dni" || test -f "$package_dir/simple-lector-dni.exe"
test -f "$package_dir/engine/simple-lector-dni-engine.jar"
test -f "$package_dir/runtime/bin/java" || test -f "$package_dir/runtime/bin/java.exe"
test -f "$package_dir/runtime/release"

while IFS= read -r path; do
  test -n "$path" || continue
  test -f "$package_dir/$path"
done < scripts/release-files.txt

expected_java_version=$(tr -d '\r\n' < "$package_dir/.java-version" | sed 's/+.*$//')
rg -Fqx "JAVA_VERSION=\"$expected_java_version\"" "$package_dir/runtime/release"

if test -x "$package_dir/simple-lector-dni"; then
  "$package_dir/simple-lector-dni" --version >/dev/null
  scripts/verify-worker-package.sh \
    "$package_dir/runtime/bin/java" \
    "$package_dir/engine/simple-lector-dni-engine.jar"
fi
