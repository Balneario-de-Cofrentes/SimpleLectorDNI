#!/usr/bin/env sh
# Builds the self-contained ZIP for the current host. Runs under sh on macOS and under
# Git Bash on Windows, so there is exactly one packaging implementation.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

scripts/build-worker.sh
cargo build --release --locked

version=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[0].version')
if test "${GITHUB_REF_TYPE:-}" = "tag" && test "$GITHUB_REF_NAME" != "v$version"; then
  echo "Release tag $GITHUB_REF_NAME does not match Cargo version $version" >&2
  exit 1
fi

. scripts/lib/host.sh
binary="simple-lector-dni$exe"

package_name="SimpleLectorDNI-v${version}-${platform}-${architecture}"
temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/simple-lector-dni.XXXXXX")
trap 'rm -rf -- "$temporary_root"' EXIT HUP INT TERM
package_dir="$temporary_root/$package_name"

mkdir -p "$package_dir/engine"
cp "target/release/$binary" "$package_dir/$binary"
cp engine/jmulticard-worker/target/simple-lector-dni-engine.jar "$package_dir/engine/"
# Manifests are read through tr so a CRLF checkout on Windows cannot corrupt paths.
tr -d '\r' < scripts/release-files.txt | while IFS= read -r source; do
  test -n "$source" || continue
  destination="$package_dir/$source"
  mkdir -p "$(dirname -- "$destination")"
  cp "$source" "$destination"
done

build_runtime "$package_dir/runtime"

scripts/verify-release-package.sh "$package_dir"
mkdir -p dist
archive="$temporary_root/$package_name.zip"
if command -v zip >/dev/null 2>&1; then
  (cd "$temporary_root" && zip -qry "$archive" "$package_name")
else
  (cd "$temporary_root" && 7z a -tzip -bso0 -bsp0 "$archive" "$package_name")
fi
mv -f "$archive" "dist/$package_name.zip"
printf '%s\n' "dist/$package_name.zip"
