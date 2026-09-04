#!/usr/bin/env sh
# Regenerates both third-party license inventories from Cargo.lock. Run after any
# change to Cargo.lock and commit the result.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

cargo about generate about.hbs --workspace --locked --fail -o THIRD_PARTY_LICENSES.html

cargo metadata --format-version 1 --locked | python3 -c '
import json, sys

metadata = json.load(sys.stdin)
members = set(metadata["workspace_members"])
version = next(p["version"] for p in metadata["packages"] if p["id"] in members)
packages = [p for p in metadata["packages"] if p["id"] not in members]
packages.sort(key=lambda p: (p["name"].lower().replace("-", "_"), p["version"]))

print("# Licencias de terceros")
print()
print("Este inventario corresponde a las dependencias Rust resueltas en `Cargo.lock` para SimpleLectorDNI " + version + ". Las expresiones de licencia proceden de los metadatos publicados por cada paquete. Los textos legales conservados por Cargo y por los proyectos de origen siguen siendo la referencia aplicable.")
print()
print("| Paquete | Versión | Licencia declarada | Origen |")
print("|---|---:|---|---|")
for p in packages:
    row = [p["name"], p["version"], p.get("license") or "", p.get("repository") or ""]
    print("| " + " | ".join(row) + " |")
' > THIRD_PARTY_LICENSES.md
