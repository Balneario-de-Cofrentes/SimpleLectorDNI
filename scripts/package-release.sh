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

mkdir -p "$package_dir/engine" "$package_dir/protocol"
cp target/release/simple-lector-dni "$package_dir/simple-lector-dni"
cp engine/jmulticard-worker/target/simple-lector-dni-engine.jar "$package_dir/engine/"
cp protocol/engine-v1.schema.json "$package_dir/protocol/"
cp README.md LICENSE RUNTIME_SOURCE.md THIRD_PARTY_NOTICES.md THIRD_PARTY_LICENSES.md \
  THIRD_PARTY_LICENSES.html "$package_dir/"

"$JAVA_HOME/bin/jlink" \
  --add-modules java.base,java.desktop,java.logging,java.naming,java.smartcardio,java.sql,jdk.crypto.ec \
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
