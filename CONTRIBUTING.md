# Contribuir

Gracias por ayudar a que la lectura de DNIe sea accesible e integrable. Ningún ejemplo, test, incidencia o pull request debe contener datos de una persona real.

## Entorno

- Rust 1.88 o posterior
- JDK 21
- Maven 3.9
- `jq`, `zip` y `jlink` para empaquetar en macOS
- PowerShell y `jlink` para empaquetar en Windows

```sh
git clone --recurse-submodules https://github.com/Balneario-de-Cofrentes/SimpleLectorDNI.git
cd SimpleLectorDNI
cargo test --workspace --locked
scripts/build-worker.sh
scripts/verify-worker-package.sh
```

Si el repositorio ya estaba clonado sin submódulos:

```sh
git submodule update --init --recursive
```

## Forma de trabajo

1. Abre una incidencia sin información personal para acordar cambios grandes.
2. Escribe primero una prueba que falle.
3. Implementa la solución más pequeña y clara.
4. Ejecuta formato, tests, Clippy y verificadores.
5. Usa un mensaje Conventional Commits cuando sea posible.

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
scripts/check-no-biometric-access.sh
scripts/check-docs-and-privacy.sh
cargo about generate about.hbs --workspace --locked --fail -o THIRD_PARTY_LICENSES.html
```

El expediente HTML de licencias debe regenerarse y versionarse cuando cambie `Cargo.lock`.

Las funciones deben ser pequeñas, los límites entre PC/SC, worker y salidas deben permanecer explícitos, y los errores públicos no deben filtrar APDU ni contenido del documento.

## Cambios en JMultiCard

El submódulo está fijado a una revisión revisada. No lo actualices de forma incidental. Un cambio requiere revisar licencia, compatibilidad de API, canal seguro, acceso a grupos de datos y pruebas del JAR empaquetado.
