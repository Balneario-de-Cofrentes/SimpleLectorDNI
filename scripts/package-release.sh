#!/usr/bin/env sh
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
architecture=$(uname -m)
case "$architecture" in
  arm64) architecture=arm64 ;;
  x86_64) architecture=x64 ;;
  *) echo "Unsupported macOS architecture: $architecture" >&2; exit 1 ;;
esac

package_name="SimpleLectorDNI-v${version}-macos-${architecture}"
temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/simple-lector-dni.XXXXXX")
trap 'rm -rf -- "$temporary_root"' EXIT HUP INT TERM
package_dir="$temporary_root/$package_name"

mkdir -p "$package_dir/engine"
cp target/release/simple-lector-dni "$package_dir/simple-lector-dni"
cp engine/jmulticard-worker/target/simple-lector-dni-engine.jar "$package_dir/engine/"
while IFS= read -r source; do
  test -n "$source" || continue
  destination="$package_dir/$source"
  mkdir -p "$(dirname -- "$destination")"
  cp "$source" "$destination"
done < scripts/release-files.txt

runtime_modules=$(tr -d '\r\n' < scripts/runtime-modules.txt)

"$JAVA_HOME/bin/jlink" \
  --add-modules "$runtime_modules" \
  --compress zip-6 \
  --strip-debug \
  --no-header-files \
  --no-man-pages \
  --output "$package_dir/runtime"

scripts/verify-release-package.sh "$package_dir"
mkdir -p dist
(cd "$temporary_root" && zip -qry "$temporary_root/$package_name.zip" "$package_name")
mv -f "$temporary_root/$package_name.zip" "dist/$package_name.zip"
printf '%s\n' "dist/$package_name.zip"
