# Código fuente del runtime Java

Los paquetes binarios de SimpleLectorDNI v0.1.x incluyen una imagen modular creada con `jlink` a partir de Eclipse Temurin `21.0.12.1+1`. La compilación queda fijada en [`.java-version`](.java-version) y el fichero `runtime/release` de cada ZIP permite comprobar la versión efectiva mediante `JAVA_VERSION`.

El código fuente correspondiente exacto es:

- Archivo: `OpenJDK21U-jdk-sources_21.0.12.1_1.tar.gz`
- Origen oficial: <https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jdk-sources_21.0.12.1_1.tar.gz>
- SHA-256: `573057d03584ae793fb7ec9a14c76d826d9187a53efeefd99da47403a5308234`

Cada release binaria de SimpleLectorDNI adjunta también ese archivo de fuentes, junto a los ZIP y `SHA256SUMS`, para que pueda descargarse desde el mismo lugar y sin coste. Los textos legales por módulo, GPL-2.0 con Classpath Exception y avisos adicionales se conservan dentro de `runtime/legal`.

SimpleLectorDNI no modifica el código fuente del JDK. `jlink` únicamente limita la imagen redistribuida a los módulos necesarios para ejecutar el worker.
