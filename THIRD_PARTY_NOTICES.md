# Third-party notices

SimpleLectorDNI uses third-party open-source components. Release bundles preserve the corresponding license texts and notices.

## JMultiCard

Copyright (C) Dirección General de Modernización Administrativa, Procedimientos e Impulso de la Administración Electrónica and its contributors.

Source: <https://github.com/ctt-gob-es/jmulticard>

Pinned version: 2.1, commit `4ec9494f181a2e94d4aeaf62b63a30bcd98f4624`

JMultiCard source headers declare dual licensing under LGPL 2.1 or later, or EUPL 1.1 or later. SimpleLectorDNI elects the EUPL licensing option. The upstream Maven metadata currently lists GPL-2.0 and EUPL-1.1. Consult the pinned upstream sources for the notices applicable to each redistributed file.

## Bouncy Castle

The JMultiCard worker redistributes Bouncy Castle `bcpkix-jdk18on`, `bcprov-jdk18on` and `bcutil-jdk18on` version 1.84.

Copyright (c) 2000-2026 The Legion of the Bouncy Castle Inc. (<https://www.bouncycastle.org>)

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## Jackson

The worker redistributes Jackson annotations 2.20, core 2.20.0 and databind 2.20.0 under Apache License 2.0.

Copyright 2007-, Tatu Saloranta and contributors.

The shaded worker JAR preserves Jackson's `META-INF/LICENSE` and `META-INF/NOTICE`, including the full Apache License 2.0 text and attribution notice.

## Runtime de Java

Los paquetes redistribuyen una imagen modular generada con `jlink` a partir de Eclipse Temurin 21. Temurin se distribuye bajo GPL-2.0 con Classpath Exception y conserva sus ficheros legales dentro del directorio `runtime/legal`. `RUNTIME_SOURCE.md` explica cómo identificar y obtener el código fuente correspondiente a la versión exacta declarada en `runtime/release`.

Fuente: <https://adoptium.net/temurin/>

## Dependencias Rust

Los nombres, versiones, expresiones de licencia y orígenes resueltos desde `Cargo.lock` se incluyen en `THIRD_PARTY_LICENSES.md`. Los avisos y textos completos detectados por `cargo-about` se incluyen en `THIRD_PARTY_LICENSES.html`. Ambos ficheros forman parte del repositorio y de cada paquete.
