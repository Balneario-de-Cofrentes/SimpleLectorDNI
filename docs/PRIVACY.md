# Guía de privacidad

SimpleLectorDNI procesa datos de identidad y domicilio. Esta guía ayuda a desplegarlo con prudencia, pero no sustituye asesoramiento jurídico ni el análisis específico de cada alojamiento.

## Principios de despliegue

- Define y documenta la base jurídica aplicable al check-in y facilita la información de privacidad al huésped.
- Recoge solo los campos que necesita el sistema de gestión. No conserves el JSON o CSV auxiliar si el PMS ya ha recibido los datos.
- Limita acceso al puesto de recepción y a cuentas autorizadas. No uses carpetas compartidas sin controles.
- Cifra discos y copias de seguridad. Usa HTTPS para cualquier webhook y rota su token.
- Establece un plazo de conservación y un borrado verificable de ficheros, logs, copias y exportaciones.
- Evita que herramientas de observabilidad capturen stdout, cuerpos HTTP o ficheros con documentos.
- Informa y forma al personal. La entrega física del documento no elimina las obligaciones de transparencia, seguridad y minimización.

## Comportamiento del programa

- Todo se procesa localmente salvo que el operador configure `--webhook`.
- No existe telemetría, cuenta remota ni servicio del proyecto.
- Los errores no incluyen los valores personales leídos.
- Los ficheros nuevos se crean con permisos `0600` en Unix.
- En Windows heredan la ACL de la carpeta. Usa una carpeta privada del usuario o una ACL corporativa restrictiva, nunca una ubicación compartida por defecto.
- `--json` usa reemplazo atómico, mientras `--jsonl` y `--csv` conservan historial por diseño.
- La URL del webhook debe usar HTTPS, excepto loopback para pruebas locales.

## Recomendación para un hotel

Prefiere `once --stdout` conectado directamente al PMS o `watch --webhook` hacia un servicio interno autenticado. Evita CSV como almacenamiento permanente. Si hace falta CSV para una transición, guárdalo en un directorio privado, impón borrado automático y registra quién puede acceder.

Antes de producción, realiza una evaluación de riesgos que cubra fallo del lector, acceso indebido al puesto, indisponibilidad del PMS, duplicados, conservación y respuesta a incidentes. Prueba siempre con datos sintéticos antes de usar documentos reales.

## Incidentes y divulgación

Si aparece un fichero con datos reales en commits, incidencias, logs o artefactos, deja de distribuirlo, restringe el acceso y aplica el procedimiento de incidentes de tu organización. No abras una incidencia pública que incluya datos de una persona.
