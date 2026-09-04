# Contribuir

Gracias por ayudar a que la lectura de DNIe sea accesible e integrable. Ningún ejemplo, test, incidencia o pull request debe contener datos de una persona real.

## Entorno

- Rust 1.88 o posterior
- JDK 21
- Maven 3.9
- `jq`, `ripgrep` y `jlink` para empaquetar; `zip` en macOS o `7z` en Windows
- Git Bash en Windows: todos los scripts son `sh`

```sh
git clone --recurse-submodules https://github.com/Balneario-de-Cofrentes/SimpleLectorDNI.git
cd SimpleLectorDNI
cargo test --workspace --locked
mvn -f engine/jmulticard-worker/pom.xml test
scripts/build-worker.sh
scripts/package-release.sh
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
scripts/generate-third-party-licenses.sh
```

Los dos inventarios de licencias se regeneran con ese script y se versionan cuando cambie `Cargo.lock`.

Las funciones deben ser pequeñas, los límites entre PC/SC, worker y salidas deben permanecer explícitos, y los errores públicos no deben filtrar APDU ni contenido del documento.

## Cambios en JMultiCard

El submódulo está fijado a una revisión revisada. No lo actualices de forma incidental. Un cambio requiere revisar licencia, compatibilidad de API, canal seguro, acceso a grupos de datos y pruebas del JAR empaquetado.
