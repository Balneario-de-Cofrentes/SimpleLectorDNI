# Historial de cambios

Los cambios relevantes de este proyecto se documentan aquí. El formato sigue Keep a Changelog y las versiones siguen Semantic Versioning.

## [Sin publicar]

### Corregido

- El worker escribe su respuesta en UTF-8 en cualquier sistema; en Windows en castellano cualquier `Ñ` o tilde hacía fallar la lectura.
- `sexo` vale `M`, `F` o vacío; antes llegaba como texto en castellano de JMultiCard.
- Fallos de firma o certificado del SOD, tarjetas que no son DNIe y retiradas a mitad de lectura ya no se reintentan ni se confunden con errores de comunicación.
- `watch` sobrevive a un reinicio del servicio PC/SC en lugar de terminar.
- El motor y las salidas se validan antes de esperar la tarjeta; un runtime ausente falla una sola vez con su ruta.
- Un desajuste entre los dos analizadores de DG13 falla con `DG13_LAYOUT` en vez de entregar campos desplazados.

### Añadido

- App de escritorio Tauri 2 en `apps/desktop`: estado de la sesión, última lectura en pantalla sin persistencia, webhook con token en el llavero del sistema, y `scripts/package-desktop.sh` para el bundle.
- Progreso en stderr, en castellano, con lector, tarjeta, intento y salida fallida; la salida estándar sigue siendo solo JSON.
- `once --timeout-seconds`.
- Reintento de webhook ante errores de red, `5xx`, `408` y `429` con la misma `Idempotency-Key`.
- Validación TLS con el almacén de certificados del sistema operativo.
- ACL exclusiva para la cuenta actual en los ficheros creados en Windows.
- Bucle de sesión único para `once` y `watch`, con eventos de progreso reutilizables por una interfaz gráfica y tests deterministas con un monitor simulado.
- Comprobación de advisories de Cargo y vigilancia de dependencias Maven y de la composite action.

### Cambiado

- Unificada la selección dinámica de lectores usada por `once` y `watch`.
- Tipados los estados de integridad y los errores seguros del motor Java; `unverified` desaparece del contrato porque nunca se emitía.
- Añadida una fixture compartida para mantener alineados Java, Rust y JSON Schema, y `mapDocument` se prueba sin hardware.
- Centralizados el pipeline CI y el manifiesto de los paquetes multiplataforma; el empaquetado es un único script `sh` también en Windows.
- Mensajes de error y de progreso en castellano; el token del webhook no se hereda en el proceso del worker.
- La documentación describe `verified` como integridad del contenido frente al certificado del SOD, sin validación de cadena CSCA.

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
