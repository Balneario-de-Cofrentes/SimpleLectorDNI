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
    "sexo": "F",
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

### Campos del documento

Todos los campos son cadenas y valen `""` cuando el chip no los contiene o no se pueden interpretar.

| Campo | Formato |
|---|---|
| `dni` | Número del documento sin separadores, tal como se lee del chip (`00000000T`). |
| `dni_formateado` | El mismo número tal como aparece en DG13, con sus separadores. |
| `fecha_nacimiento`, `fecha_caducidad` | ISO 8601 (`AAAA-MM-DD`). Una fecha no interpretable queda vacía. |
| `sexo` | `M`, `F` o vacío. |
| `nacionalidad`, `pais`, `pais_nacimiento` | Como los escribe el chip. `pais_nacimiento` lo deriva JMultiCard de la provincia de nacimiento. |
| `apellidos` | `primer_apellido` y `segundo_apellido` unidos por un espacio. |
| `version_dnie`, `serial_chip` | Datos técnicos opcionales; vacíos si no se pueden leer. |

### Integridad

`integrity.sod_signature` e `integrity.dg13_hash` solo pueden valer `verified`. El worker aborta cualquier lectura en la que la firma del SOD o el hash de DG13 no se verifiquen, con el código `INTEGRITY_ERROR`, así que nunca se entrega un registro con integridad dudosa. `verified` significa que el hash de DG13 coincide con el firmado y que la firma CMS es válida respecto al certificado incluido en el SOD. No se valida ese certificado contra la CSCA de la Dirección General de la Policía.

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

Solo se aceptan URLs HTTPS. HTTP se permite exclusivamente para `localhost` o una dirección loopback durante desarrollo. Los certificados se validan con el almacén de confianza del sistema operativo, así que un PMS con certificado de una CA corporativa funciona si esa CA está instalada en el equipo. No se siguen redirecciones. El timeout predeterminado es de 10 segundos y se cambia con `--webhook-timeout-seconds`.

Un error de red, un `5xx`, un `408` o un `429` se reintenta hasta tres veces con la misma `Idempotency-Key`. Cualquier otro `4xx` se considera definitivo. El receptor debe deduplicar usando `Idempotency-Key`.

Cada salida está aislada. Si una falla, el programa intenta las restantes, escribe en stderr `Salida <nombre> fallida: <motivo>` y en modo `once` devuelve error. En modo `watch` la lectura no se reintenta, así que conviene combinar `--webhook` con `--jsonl` como diario local para recuperar lecturas si el PMS no estaba disponible.

## Lectores y proceso

```sh
simple-lector-dni list-readers
simple-lector-dni once --reader "parte del nombre" --attempts 3 --timeout-seconds 60
simple-lector-dni watch --reader "parte del nombre" --retry-delay-ms 350
```

`once` espera lector y tarjeta, entrega una lectura y termina; con `--timeout-seconds` falla si no se inserta un DNIe en ese plazo. `watch` espera indefinidamente y requiere una retirada entre dos lecturas. `--attempts` acepta de 1 a 3. El motor y las salidas se validan antes de esperar la tarjeta, de modo que una instalación rota falla sin que el huésped haya entregado el documento.

La salida estándar solo contiene JSON. En stderr se escribe el progreso en castellano (lector esperado, tarjeta esperada, intento en curso, lectura entregada, tarjeta pendiente de retirar) y los errores, nunca datos del documento. Un proceso correcto devuelve código 0. Los fallos de configuración, lectura agotada, timeout o entrega devuelven un código distinto de 0 en modo `once`. En `watch`, un fallo de lectura se informa y el proceso continúa esperando una retirada. Si el servicio PC/SC deja de responder, `watch` lo reintenta cada segundo hasta recuperarlo.

Códigos de fallo de lectura:

| Código | Reintento | Significado |
|---|---|---|
| `CARD_READ_FAILED` | sí | Error de comunicación con la tarjeta. |
| `READER_NOT_FOUND` | sí | El lector seleccionado no está disponible para el worker. |
| `CARD_REMOVED` | no | El DNIe se retiró durante la lectura. |
| `UNSUPPORTED_CARD` | no | La tarjeta no es un DNIe compatible. |
| `INTEGRITY_ERROR` | no | Firma del SOD, certificado del SOD o hash de DG13 no verificables. |
| `DG13_LAYOUT` | no | Los dos analizadores de DG13 no coinciden; no se entrega ningún campo. |
| `ENGINE_NOT_FOUND`, `ENGINE_START_FAILED` | no | El runtime Java o el worker no están donde el paquete los espera. |
| `ENGINE_TIMEOUT`, `ENGINE_EXIT_<n>` | sí | El worker no respondió a tiempo o terminó con ese código de salida. |

## Motor reemplazable

El supervisor habla con el worker mediante JSON por stdin y stdout conforme a [engine-v1.schema.json](../protocol/engine-v1.schema.json). El worker escribe siempre UTF-8, con independencia del juego de caracteres del sistema, y [success.json](../protocol/examples/success.json) es la respuesta de ejemplo que comparten los tests de Rust y Java. El proceso del worker no hereda `SIMPLE_LECTOR_DNI_WEBHOOK_TOKEN`. En desarrollo se pueden usar `SIMPLE_LECTOR_DNI_JAVA` y `SIMPLE_LECTOR_DNI_ENGINE_JAR` para probar otra JVM u otro JAR. Los paquetes publicados resuelven rutas relativas y no necesitan estas variables.
