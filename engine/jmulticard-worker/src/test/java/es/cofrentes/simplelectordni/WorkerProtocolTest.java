package es.cofrentes.simplelectordni;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

final class WorkerProtocolTest {

    private static final ObjectMapper JSON = new ObjectMapper();

    @Test
    void returnsVersionedSuccessForAValidRequest() throws Exception {
        final DocumentData document = new DocumentData();
        document.nombre = "ANA";
        document.dni = "00000000T";
        final DniReader reader = readerName -> {
            assertEquals("Synthetic reader", readerName);
            return new DniReadResult(
                document,
                new IntegrityResult("verified", "verified")
            );
        };

        final String response = Worker.handle(
            "{\"protocol\":1,\"command\":\"read\",\"reader_name\":\"Synthetic reader\"}",
            reader
        );
        final JsonNode json = JSON.readTree(response);

        assertEquals(1, json.path("protocol").asInt());
        assertEquals("ok", json.path("status").asText());
        assertEquals("ANA", json.path("document").path("nombre").asText());
        assertEquals("00000000T", json.path("document").path("dni").asText());
        assertEquals("verified", json.path("integrity").path("dg13_hash").asText());
    }

    @Test
    void rejectsAnUnsupportedProtocolWithoutCallingTheReader() throws Exception {
        final String response = Worker.handle(
            "{\"protocol\":99,\"command\":\"read\",\"reader_name\":\"Synthetic reader\"}",
            readerName -> { throw new AssertionError("reader must not be called"); }
        );

        final JsonNode json = JSON.readTree(response);
        assertEquals("error", json.path("status").asText());
        assertEquals("INVALID_REQUEST", json.path("error").path("code").asText());
        assertFalse(json.path("error").path("retryable").asBoolean());
    }

    @Test
    void unexpectedErrorsNeverLeakIdentityData() throws Exception {
        final String response = Worker.handle(
            "{\"protocol\":1,\"command\":\"read\",\"reader_name\":\"Synthetic reader\"}",
            readerName -> { throw new IllegalStateException("sensitive DNI 00000000T"); }
        );

        assertFalse(response.contains("00000000T"));
        final JsonNode json = JSON.readTree(response);
        assertEquals("INTERNAL_ERROR", json.path("error").path("code").asText());
        assertTrue(json.path("error").path("retryable").asBoolean());
    }

    @Test
    void expectedReadErrorsNeverLeakTheirOriginalMessage() throws Exception {
        final String response = Worker.handle(
            "{\"protocol\":1,\"command\":\"read\",\"reader_name\":\"Synthetic reader\"}",
            readerName -> {
                throw new DniReadException(
                    "CARD_READ_FAILED",
                    "sensitive DNI 00000000T",
                    true,
                    null
                );
            }
        );

        assertFalse(response.contains("00000000T"));
        final JsonNode json = JSON.readTree(response);
        assertEquals("CARD_READ_FAILED", json.path("error").path("code").asText());
        assertEquals("DNIe read failed", json.path("error").path("message").asText());
    }

    @Test
    void invalidErrorCodesAreSanitizedBeforeSerialization() throws Exception {
        final String response = Worker.handle(
            "{\"protocol\":1,\"command\":\"read\",\"reader_name\":\"Synthetic reader\"}",
            readerName -> {
                throw new DniReadException("DNI_00000000T", "failure", true, null);
            }
        );

        assertFalse(response.contains("00000000T"));
        assertEquals(
            "INTERNAL_ERROR",
            JSON.readTree(response).path("error").path("code").asText()
        );
    }
}
