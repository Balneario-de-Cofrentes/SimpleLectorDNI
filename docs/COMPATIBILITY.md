# Compatibilidad

## Plataformas distribuidas

| Plataforma | Arquitectura | Compilación y tests | Prueba con hardware |
|---|---|---:|---:|
| macOS | Apple Silicon ARM64 | Sí | Sí |
| macOS | Intel x64 | Sí, CI | Pendiente de ampliar |
| Windows | x64 | Sí, CI | Pendiente de ampliar |

Los paquetes publicados incluyen el binario, el worker JMultiCard y un runtime Java mínimo. El sistema debe ofrecer PC/SC. En Windows lo proporciona el servicio de tarjeta inteligente. En macOS forma parte del sistema.

La beta no tiene firma de código ni notarización. Gatekeeper o SmartScreen pueden solicitar confirmación, y un entorno corporativo puede bloquearla. Los checksums permiten verificar la descarga, pero no sustituyen una firma de editor.

## Matriz física verificada

Prueba realizada el 4 de septiembre de 2026:

| Sistema | Lector | USB | Resultado |
|---|---|---|---|
| macOS 26.3 ARM64 | Generic EMV Smartcard Reader, Alcor AU9540 | `058f:9540` | DG13 completo, firma SOD y hash DG13 verificados |

La prueba confirmó los 21 campos del contrato con contenido en el documento usado, sin registrar sus valores. También se verificó el paquete autocontenido de macOS, no solo el entorno de desarrollo.

## Verificación del SOD

La firma del SOD se comprueba respecto al certificado que el propio SOD incluye, sin validar ese certificado contra la CSCA ni comprobar su vigencia. Un fallo de firma o de certificado devuelve `INTEGRITY_ERROR` y no se reintenta. Un DNIe cuyo certificado de firma del documento haya caducado sigue leyéndose mientras la firma sea correcta; la política sobre certificados caducados queda pendiente hasta que exista validación de cadena.

## Juego de caracteres

El worker escribe su respuesta en UTF-8 con independencia del juego de caracteres del sistema. En la beta anterior, Windows en castellano (`Cp1252`) hacía fallar cualquier lectura con `Ñ` o tildes; la corrección tiene test unitario pero la prueba física en Windows con un documento así sigue pendiente.

## Documentos

La implementación está orientada a DNIe compatible con la clase `Dnie3` de JMultiCard y su canal seguro de usuario. No incluye pasaportes, NIE/TIE ópticos, documentos extranjeros ni DNI antiguos sin chip compatible.

## Cómo aportar compatibilidad

Ejecuta la lista de [pruebas manuales](MANUAL_TESTS.md), elimina cualquier dato personal de los resultados y abre una incidencia con:

- sistema operativo y arquitectura;
- nombre del lector mostrado por `list-readers`;
- VID:PID si está disponible;
- versión visible del DNIe, sin número ni datos personales;
- código de error sanitizado o resultado correcto.

No existe una garantía universal para cualquier lector que anuncie PC/SC. La matriz se ampliará únicamente con evidencia reproducible.
