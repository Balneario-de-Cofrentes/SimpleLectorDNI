# Pruebas manuales

No publiques salidas completas de documentos reales. Valida estructura y presencia con `jq`, y elimina los ficheros al terminar según la política local.

## Lista por plataforma y lector

1. Ejecutar `simple-lector-dni list-readers` sin tarjeta y confirmar que aparece el lector.
2. Ejecutar `simple-lector-dni once --stdout`, insertar el DNIe y comprobar `schema_version`, `source` e integridad sin imprimir `document`.
3. Ejecutar `once` con `--json`, `--jsonl` y `--csv` combinados. Confirmar un registro, 28 columnas y permisos privados donde aplique.
4. Ejecutar `watch --jsonl prueba.jsonl`. Retirar e insertar la tarjeta y confirmar una única línea nueva por inserción.
5. Provocar una lectura fallida transitoria y comprobar un máximo de tres intentos.
6. Desconectar y reconectar el lector durante `watch` y verificar recuperación.
7. Probar un webhook loopback que compruebe esquema, `Idempotency-Key` y respuesta sin guardar el cuerpo.
8. Probar el ZIP publicado en una máquina limpia, sin Java instalado por separado.
9. Leer un documento cuyo nombre o apellidos contengan `Ñ` o tildes y comprobar que el campo llega con el carácter correcto. Imprescindible en Windows.
10. Ejecutar `once --timeout-seconds 5` sin tarjeta y comprobar que termina con error pasado el plazo.
11. Durante `watch`, reiniciar el servicio de tarjeta inteligente (Windows: `Restart-Service SCardSvr`; macOS: desconectar el lector con la tarjeta dentro y volver a conectarlo) y confirmar que aparece `Servicio PC/SC recuperado` y que la siguiente inserción se lee.
12. Apuntar `--webhook` a un receptor loopback que devuelva `503` una vez y `204` después, y confirmar una sola lectura con dos peticiones y la misma `Idempotency-Key`.
13. Comprobar en stderr que el progreso indica lector, tarjeta, intento y salida fallida sin ningún dato del documento.

Comprobación segura de una lectura:

```sh
simple-lector-dni once --stdout | jq '{schema_version, source, integrity, campos_con_valor: ([.document[] | select(. != "")] | length)}'
```

## Evidencia de la versión 0.1.0

En macOS 26.3 ARM64 con Alcor AU9540 se verificaron:

- detección PC/SC del lector y la tarjeta;
- lectura DG13 mediante el binario Rust y el worker JMultiCard;
- firma SOD y hash DG13 con estado `verified`;
- 21 campos con contenido en el documento de prueba;
- JSON, JSON Lines y CSV con 28 columnas;
- dos inserciones consecutivas en `watch`, con dos IDs distintos y sin duplicados;
- webhook loopback con esquema, integridad e `Idempotency-Key` correctos;
- permisos `0600` en los ficheros creados;
- ZIP autocontenido de macOS ejecutado fuera del árbol de compilación.

Los valores personales no forman parte del repositorio, de los logs de prueba ni de esta evidencia.
