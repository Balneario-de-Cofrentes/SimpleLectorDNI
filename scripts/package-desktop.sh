#!/usr/bin/env sh
# Stages the Java runtime and the worker as Tauri resources and builds the desktop
# bundle (DMG on macOS, MSI/NSIS on Windows) into target/release/bundle.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
. scripts/lib/host.sh

scripts/build-worker.sh

resources=apps/desktop/resources
rm -rf "$resources"
mkdir -p "$resources/engine"
cp engine/jmulticard-worker/target/simple-lector-dni-engine.jar "$resources/engine/"
build_runtime "$resources/runtime"
scripts/verify-worker-package.sh \
  "$resources/runtime/bin/java$exe" \
  "$resources/engine/simple-lector-dni-engine.jar"

(cd apps/desktop && cargo tauri build "$@")
