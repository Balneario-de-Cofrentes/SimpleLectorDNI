package es.cofrentes.simplelectordni;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.charset.StandardCharsets;

import org.junit.jupiter.api.Test;

final class Dg13TextFieldsTest {

    @Test
    void formatsPresentDatesWithoutUsingJmulticardFallbacks() {
        final Dg13TextFields fields = Dg13TextFields.from(encodedDg13(
            "HEADER", "SURNAME", "SECOND", "NAME", "DOCUMENT", "31 12 1990",
            "ESP", "01 01 2030"
        ));

        assertEquals("1990-12-31", fields.isoDateAt(5));
        assertEquals("2030-01-01", fields.isoDateAt(7));
    }

    @Test
    void missingOrMalformedDatesStayEmpty() {
        final Dg13TextFields missing = Dg13TextFields.from(encodedDg13("HEADER"));
        final Dg13TextFields malformed = Dg13TextFields.from(encodedDg13(
            "HEADER", "SURNAME", "SECOND", "NAME", "DOCUMENT", "31 02 2026"
        ));

        assertEquals("", missing.isoDateAt(5));
        assertEquals("", malformed.isoDateAt(5));
    }

    @Test
    void ignoresARealisticLongFormDerHeader() {
        final Dg13TextFields fields = Dg13TextFields.from(encodedDg13(
            "HEADER", "SURNAME", "SECOND", "NAME", "DOCUMENT", "31 12 1990",
            "ESP", "01 01 2030", "SUPPORT", "M", "CITY", "PROVINCE", "PARENTS",
            "X".repeat(300)
        ));

        assertEquals("1990-12-31", fields.isoDateAt(5));
        assertEquals("2030-01-01", fields.isoDateAt(7));
    }

    @Test
    void ignoresASingleOctetLongFormDerHeader() {
        final Dg13TextFields fields = Dg13TextFields.from(encodedDg13(
            "HEADER", "SURNAME", "SECOND", "NAME", "DOCUMENT", "31 12 1990",
            "ESP", "01 01 2030", "X".repeat(140)
        ));

        assertEquals("1990-12-31", fields.isoDateAt(5));
        assertEquals("2030-01-01", fields.isoDateAt(7));
    }

    @Test
    void malformedDerNeverLeaksAReplacementDate() {
        final Dg13TextFields fields = Dg13TextFields.from(
            new byte[] {0x6D, (byte) 0x82, 0x02}
        );

        assertEquals("", fields.isoDateAt(5));
        assertEquals("", fields.isoDateAt(7));
    }

    private static byte[] encodedDg13(final String... fields) {
        final byte[] value = String.join("\u0000\u0000", fields)
            .getBytes(StandardCharsets.UTF_8);
        final int lengthOctets = value.length < 128 ? 1 : value.length < 256 ? 2 : 3;
        final byte[] encoded = new byte[1 + lengthOctets + value.length];
        encoded[0] = 0x6D;
        if (lengthOctets == 1) {
            encoded[1] = (byte) value.length;
        }
        else if (lengthOctets == 2) {
            encoded[1] = (byte) 0x81;
            encoded[2] = (byte) value.length;
        }
        else {
            encoded[1] = (byte) 0x82;
            encoded[2] = (byte) (value.length >> 8);
            encoded[3] = (byte) value.length;
        }
        System.arraycopy(value, 0, encoded, 1 + lengthOctets, value.length);
        return encoded;
    }
}
