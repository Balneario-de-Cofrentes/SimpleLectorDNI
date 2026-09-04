package es.cofrentes.simplelectordni;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.charset.StandardCharsets;

import org.junit.jupiter.api.Test;

final class Dg13TextFieldsTest {

    @Test
    void formatsPresentDatesWithoutUsingJmulticardFallbacks() {
        final Dg13TextFields fields = Dg13TextFields.from(encodedFields(
            "HEADER", "SURNAME", "SECOND", "NAME", "DOCUMENT", "31 12 1990",
            "ESP", "01 01 2030"
        ));

        assertEquals("1990-12-31", fields.isoDateAt(5));
        assertEquals("2030-01-01", fields.isoDateAt(7));
    }

    @Test
    void missingOrMalformedDatesStayEmpty() {
        final Dg13TextFields missing = Dg13TextFields.from(encodedFields("HEADER"));
        final Dg13TextFields malformed = Dg13TextFields.from(encodedFields(
            "HEADER", "SURNAME", "SECOND", "NAME", "DOCUMENT", "31 02 2026"
        ));

        assertEquals("", missing.isoDateAt(5));
        assertEquals("", malformed.isoDateAt(5));
    }

    private static byte[] encodedFields(final String... fields) {
        return String.join("\u0000\u0000", fields).getBytes(StandardCharsets.UTF_8);
    }
}
