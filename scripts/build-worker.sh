#!/usr/bin/env sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

maven=${MVN:-mvn}

git submodule update --init --recursive
"$maven" -q -f vendor/jmulticard/pom.xml -DskipTests install
"$maven" -q -f engine/jmulticard-worker/pom.xml package

scripts/verify-worker-package.sh \
  "$JAVA_HOME/bin/java" \
  engine/jmulticard-worker/target/simple-lector-dni-engine.jar
