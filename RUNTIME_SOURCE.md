# Código fuente del runtime Java

Los paquetes binarios de SimpleLectorDNI incluyen una imagen modular creada con `jlink` a partir de Eclipse Temurin 21. El fichero `runtime/release` de cada paquete identifica la versión exacta mediante `JAVA_VERSION` y enumera los módulos incluidos.

El código fuente correspondiente de OpenJDK/Temurin se publica sin coste en:

- <https://github.com/adoptium/jdk21u>
- <https://github.com/adoptium/temurin21-binaries/releases>

Selecciona la etiqueta o el archivo de fuentes que corresponda al `JAVA_VERSION` incluido en el paquete. Los textos legales por módulo, GPL-2.0 con Classpath Exception y avisos de componentes adicionales se conservan dentro de `runtime/legal`.

SimpleLectorDNI no modifica el código fuente del JDK. `jlink` limita la imagen redistribuida a los módulos necesarios para ejecutar el worker.
