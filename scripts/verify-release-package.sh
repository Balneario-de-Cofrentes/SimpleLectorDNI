#!/usr/bin/env sh
set -eu

package_dir=${1:?"usage: verify-release-package.sh <package-directory>"}

require() {
  "$@" || { echo "release package check failed: $*" >&2; exit 1; }
}

if test -f "$package_dir/simple-lector-dni.exe"; then
  binary="$package_dir/simple-lector-dni.exe"
  java="$package_dir/runtime/bin/java.exe"
else
  binary="$package_dir/simple-lector-dni"
  java="$package_dir/runtime/bin/java"
fi

require test -x "$binary"
require test -f "$package_dir/engine/simple-lector-dni-engine.jar"
require test -x "$java"
require test -f "$package_dir/runtime/release"

tr -d '\r' < scripts/release-files.txt | while IFS= read -r path; do
  test -n "$path" || continue
  require test -f "$package_dir/$path"
done

expected_java_version=$(tr -d '\r\n' < "$package_dir/.java-version" | sed 's/+.*$//')
require rg -Fqx "JAVA_VERSION=\"$expected_java_version\"" "$package_dir/runtime/release"

require "$binary" --version
require scripts/verify-worker-package.sh "$java" "$package_dir/engine/simple-lector-dni-engine.jar"
