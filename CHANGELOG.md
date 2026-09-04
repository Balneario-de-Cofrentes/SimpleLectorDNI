# Historial de cambios

Los cambios relevantes de este proyecto se documentan aquí. El formato sigue Keep a Changelog y las versiones siguen Semantic Versioning.

## [0.1.0] - 2026-09-04

### Añadido

- CLI Rust con modos `list-readers`, `once` y `watch`.
- Detección PC/SC de lector, inserción, retirada y reconexión.
- Detección mediante contador PC/SC de retiradas y reinserciones ocurridas durante una lectura.
- Hasta tres reintentos configurables por inserción.
- Lectura de 21 campos textuales de DG13 mediante JMultiCard 2.1.
- Verificación de firma SOD y hash de DG13 sin cargar fotografía o firma manuscrita.
- Salidas combinables por stdout, JSON atómico, JSON Lines, CSV y webhook HTTPS.
- Selección estable del lector por nombre exacto entre los procesos Rust y Java.
- Webhooks sin redirecciones y con aceptación exclusiva de respuestas 2xx.
- Protección contra fórmulas en CSV, ficheros privados en Unix e idempotencia de webhook.
- Paquetes autocontenidos para macOS ARM64, macOS Intel y Windows x64.
- Tests unitarios, de integración, contrato, empaquetado y CI multiplataforma.
