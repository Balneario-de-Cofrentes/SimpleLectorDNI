# Recursos del bundle

`scripts/package-desktop.sh` deja aquí `runtime/` (imagen `jlink` de Temurin) y
`engine/simple-lector-dni-engine.jar` antes de ejecutar `cargo tauri build`. Ambos se
copian al bundle bajo `resources/` y la app los resuelve con `ProcessEngine::bundled_layout`.
No se versionan; este fichero existe para que la compilación sin bundle encuentre el glob.
