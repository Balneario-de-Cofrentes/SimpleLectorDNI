#!/usr/bin/env sh
# Stages the Java runtime and the worker as Tauri resources and builds the desktop
# bundle (DMG on macOS, MSI/NSIS on Windows) into target/release/bundle.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
. scripts/lib/host.sh

scripts/build-worker.sh

# Only the staged runtime and worker are replaced; resources/README.md stays tracked so
# a plain build (no bundle) still satisfies the resources glob in tauri.conf.json.
resources=apps/desktop/resources
rm -rf "$resources/runtime" "$resources/engine"
mkdir -p "$resources/engine"
cp engine/jmulticard-worker/target/simple-lector-dni-engine.jar "$resources/engine/"
build_runtime "$resources/runtime"
scripts/verify-worker-package.sh \
  "$resources/runtime/bin/java$exe" \
  "$resources/engine/simple-lector-dni-engine.jar"

(cd apps/desktop && cargo tauri build "$@")
