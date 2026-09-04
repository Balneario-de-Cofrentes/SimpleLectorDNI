# Investigación técnica

## Objetivo y alcance

El objetivo es extraer con consentimiento los datos textuales que el DNIe ya ofrece a aplicaciones autorizadas para un check-in, sin fotografiar el documento, hacer OCR o firmar. La solución usa la interfaz estándar PC/SC del sistema operativo y un lector de contacto compatible.

## Por qué JMultiCard

[JMultiCard](https://github.com/ctt-gob-es/jmulticard) es un proyecto publicado por el Centro de Transferencia de Tecnología de la Administración española. Implementa tarjetas españolas, incluido DNIe, y abstrae APDU, canal seguro y estructuras LDS.

SimpleLectorDNI fija la versión `2.1` como submódulo para que una compilación futura no cambie de comportamiento sin revisión. El worker abre el canal de usuario mediante `Dnie3.openSecureChannelIfNotAlreadyOpened(false)`, es decir, sin acceso de firma y sin PIN.

## Flujo de lectura

1. Rust detecta lectores y estados de tarjeta mediante PC/SC.
2. El worker Java selecciona el lector y fuerza T=0, protocolo requerido por la ruta DNIe probada.
3. JMultiCard establece el canal seguro de usuario con el chip.
4. Se selecciona y lee DG13.
5. `OptionalDetails` transforma DG13 en campos textuales.
6. Se valida la firma del SOD y se compara el hash publicado para DG13.
7. El worker devuelve únicamente el contrato JSON v1.

La documentación oficial del DNIe describe los mecanismos de acceso BAC y PACE en su [documentación técnica](https://www.dnielectronico.es/PortalDNIe/PRF1_Cons02.action?pag=REF_230).

## Datos y biometría

DG13 contiene los datos textuales mostrados por el programa, incluida la dirección y el número de soporte cuando están presentes. DG2 contiene la fotografía y DG7 la firma manuscrita. El código de producción tiene una comprobación automática que prohíbe llamadas a `getDg2()` o `getDg7()`.

La verificación del SOD es selectiva. Valida su firma y el hash de DG13 sin cargar otros grupos de datos. Esto confirma que el contenido textual entregado coincide con lo firmado en el documento. La firma se valida contra el certificado que el SOD transporta; JMultiCard documenta que `validateCmsSignature` no comprueba la validez de ese certificado y devuelve la cadena para una validación externa que SimpleLectorDNI todavía no hace. Un fallo de firma o de certificado se devuelve como `INTEGRITY_ERROR` y no se reintenta.

DG13 se interpreta dos veces. `OptionalDetails` de JMultiCard aporta los campos de texto, pero divide los bytes sin quitar la cabecera DER y devuelve la fecha actual cuando falta una fecha, así que las fechas se extraen con un analizador propio que quita la cabecera y devuelve vacío ante cualquier duda. Si ambos analizadores no coinciden en el número de documento, la lectura falla con `DG13_LAYOUT` en lugar de entregar campos desplazados.

## Qué aporta el hardware

Para este alcance, el lector actúa como interfaz eléctrica y PC/SC. No necesita reconocer visualmente el documento. Eso hace viable usar lectores USB económicos compatibles con tarjetas inteligentes.

No todos los equipos de precio alto son equivalentes a un lector PC/SC. Algunos añaden OCR de pasaportes y documentos extranjeros, captura óptica, iluminación UV o IR, lectura NFC, detección de falsificaciones, soporte y certificaciones. SimpleLectorDNI no reproduce esas funciones y no debe anunciarse como un sistema antifraude.

## Decisiones de arquitectura

- Rust mantiene un único binario supervisor, sin interfaz gráfica, fácil de integrar y con bajo consumo. El bucle de sesión emite eventos de progreso que la CLI imprime en stderr y que una interfaz gráfica puede consumir sin cambiar la lógica.
- Java queda aislado en un worker pequeño porque JMultiCard es la implementación de referencia más completa localizada para DNIe.
- Los paquetes incluyen un runtime Java recortado con `jlink`, por lo que el usuario final no instala Java.
- El contrato JSON separa la integración del motor. Una futura implementación nativa puede sustituirlo sin romper PMS, CSV o webhooks.
- Los errores cruzan la frontera como códigos sanitizados y nunca como volcados de tarjeta.

Los detalles de código relevantes se pueden revisar en [`DniReader`](../engine/jmulticard-worker/src/main/java/es/cofrentes/simplelectordni/DniReader.java) y [`Dg13Reader`](../engine/jmulticard-worker/src/main/java/es/cofrentes/simplelectordni/Dg13Reader.java).
