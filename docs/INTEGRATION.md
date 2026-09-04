# Integración

## Contrato JSON v1

Cada lectura produce un objeto con identificador idempotente, instante con zona horaria, lector, origen, resultados de integridad y los datos textuales de DG13.

```json
{
  "schema_version": 1,
  "read_id": "018f20ab-7f36-7d4f-9ec5-9ed0f75f4f83",
  "read_at": "2026-09-04T12:34:56+02:00",
  "reader": "Lector PCSC de ejemplo",
  "source": "DNIe_DG13",
  "integrity": {
    "sod_signature": "verified",
    "dg13_hash": "verified"
  },
  "document": {
    "nombre": "PERSONA",
    "primer_apellido": "EJEMPLO",
    "segundo_apellido": "PRUEBA",
    "apellidos": "EJEMPLO PRUEBA",
    "dni": "00000000T",
    "dni_formateado": "00000000-T",
    "fecha_nacimiento": "1990-01-01",
    "nacionalidad": "ESP",
    "fecha_caducidad": "2030-01-01",
    "numero_soporte": "SOPORTE-DEMO",
    "sexo": "X",
    "ciudad_nacimiento": "CIUDAD",
    "provincia_nacimiento": "PROVINCIA",
    "pais_nacimiento": "ESP",
    "nombres_progenitores": "DATOS DE PRUEBA",
    "direccion": "DIRECCION DE PRUEBA",
    "localidad": "LOCALIDAD",
    "provincia": "PROVINCIA",
    "pais": "ESP",
    "version_dnie": "3.0",
    "serial_chip": "SERIAL-DEMO"
  }
}
```

Los consumidores deben ignorar campos desconocidos para permitir ampliaciones compatibles. Un cambio incompatible incrementará `schema_version`.

## JSON, JSON Lines y CSV

```sh
simple-lector-dni once --json ultimo.dni.json
simple-lector-dni watch --jsonl historial.jsonl
simple-lector-dni watch --csv historial.csv
```

`--json` representa siempre la última lectura completa y se reemplaza de forma atómica. `--jsonl` y `--csv` son historiales append-only mientras el proceso está activo. Cada nueva inserción correcta añade exactamente un registro.

El CSV contiene 28 columnas, en este orden:

```text
schema_version,read_id,read_at,reader,source,integrity_sod_signature,nombre,primer_apellido,segundo_apellido,apellidos,dni,dni_formateado,fecha_nacimiento,nacionalidad,fecha_caducidad,numero_soporte,sexo,ciudad_nacimiento,provincia_nacimiento,pais_nacimiento,nombres_progenitores,direccion,localidad,provincia,pais,version_dnie,serial_chip,integrity_dg13_hash
```

## Webhook

```sh
export SIMPLE_LECTOR_DNI_WEBHOOK_TOKEN='token-gestionado-fuera-del-script'
simple-lector-dni watch --webhook https://pms.example.com/check-in/dni
```

La petición usa `POST` con JSON e incluye:

- `Content-Type: application/json`
- `Idempotency-Key: <read_id>`
- `User-Agent: SimpleLectorDNI/<version>`
- `Authorization: Bearer <token>`, solo si existe `SIMPLE_LECTOR_DNI_WEBHOOK_TOKEN`

Solo se aceptan URLs HTTPS. HTTP se permite exclusivamente para `localhost` o una dirección loopback durante desarrollo. No se siguen redirecciones. El timeout predeterminado es de 10 segundos y se cambia con `--webhook-timeout-seconds`.

Cada salida está aislada. Si una falla, el programa intenta las restantes y devuelve error para que supervisión pueda detectarlo. El receptor debe deduplicar usando `Idempotency-Key`.

## Lectores y proceso

```sh
simple-lector-dni list-readers
simple-lector-dni once --reader "parte del nombre" --attempts 3
simple-lector-dni watch --reader "parte del nombre" --retry-delay-ms 350
```

`once` espera lector y tarjeta, entrega una lectura y termina. `watch` espera indefinidamente y requiere una retirada entre dos lecturas. `--attempts` acepta de 1 a 3.

Los errores se escriben en stderr sin incluir datos del documento. Un proceso correcto devuelve código 0. Los fallos de configuración, lectura agotada o entrega devuelven un código distinto de 0 en modo `once`. En `watch`, un fallo de lectura se informa y el proceso continúa esperando una retirada.

## Motor reemplazable

El supervisor habla con el worker mediante JSON por stdin y stdout conforme a [engine-v1.schema.json](../protocol/engine-v1.schema.json). En desarrollo se pueden usar `SIMPLE_LECTOR_DNI_JAVA` y `SIMPLE_LECTOR_DNI_ENGINE_JAR` para probar otra JVM u otro JAR. Los paquetes publicados resuelven rutas relativas y no necesitan estas variables.
