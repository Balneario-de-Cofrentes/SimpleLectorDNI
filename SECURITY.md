# Seguridad

## Versiones soportadas

La rama `main` y la versión beta más reciente reciben correcciones. El proyecto todavía no declara estabilidad de seguridad para entornos sin una evaluación propia del integrador.

## Informar de una vulnerabilidad

Usa de forma privada la opción **Report a vulnerability** en la pestaña Security del repositorio. No abras una incidencia pública si el informe contiene una vulnerabilidad explotable, un token, datos de DNI o información de huéspedes.

Incluye versión, plataforma, lector, pasos mínimos con datos sintéticos e impacto. Se confirmará la recepción y se coordinará una publicación cuando exista una solución.

## Fronteras de confianza

- El lector USB, su firmware y el servicio PC/SC son entradas no confiables.
- El worker Java se ejecuta como un proceso separado y solo intercambia JSON versionado.
- Los ficheros y stdout contienen datos personales y deben protegerse fuera del proceso.
- Un webhook cruza la máquina local y requiere HTTPS, autenticación, autorización y una política de retención del receptor.
- El token del webhook procede de una variable de entorno y nunca debe escribirse en argumentos, configuración versionada o logs.

SimpleLectorDNI no es una herramienta antifraude y no certifica la autenticidad física del soporte. La verificación del SOD comprueba que DG13 coincide con el hash firmado y que la firma es válida respecto al certificado incluido en el propio SOD; no valida ese certificado contra la CSCA, por lo que un chip que emulara el sistema de ficheros con un certificado propio pasaría la comprobación. Esa validación de cadena es una decisión de producto pendiente. Nada de esto sustituye los controles operativos o legales del alojamiento.
