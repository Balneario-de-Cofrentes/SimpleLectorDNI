#!/usr/bin/env sh
set -eu

for path in \
  README.md \
  RUNTIME_SOURCE.md \
  CHANGELOG.md \
  CONTRIBUTING.md \
  SECURITY.md \
  THIRD_PARTY_LICENSES.md \
  THIRD_PARTY_LICENSES.html \
  docs/INTEGRATION.md \
  docs/RESEARCH.md \
  docs/PRIVACY.md \
  docs/COMPATIBILITY.md \
  docs/MANUAL_TESTS.md
do
  test -s "$path"
done

rg -q 'Licencias Rust de terceros' THIRD_PARTY_LICENSES.html

rg -q 'simple-lector-dni once' README.md
rg -q 'simple-lector-dni watch' README.md
rg -q 'simple-lector-dni list-readers' README.md
rg -q -- '--jsonl' docs/INTEGRATION.md
rg -q -- '--webhook' docs/INTEGRATION.md

if rg -n 'gh[opsu]_[A-Za-z0-9]{20,}|-----BEGIN [A-Z ]*PRIVATE KEY-----' \
  --glob '!vendor/**' --glob '!target/**' .; then
  echo "Potential secret found in tracked project files" >&2
  exit 1
fi

unexpected_dni=$(rg -n '[0-9]{8}[A-Z]' --glob '!vendor/**' --glob '!target/**' . || true)
if test -n "$unexpected_dni" && printf '%s\n' "$unexpected_dni" | rg -v '00000000T'; then
  echo "Potential non-synthetic DNI found in project files" >&2
  exit 1
fi
