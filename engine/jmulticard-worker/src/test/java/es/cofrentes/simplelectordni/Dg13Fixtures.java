package es.cofrentes.simplelectordni;

import java.nio.charset.StandardCharsets;

/** Synthetic DG13 payloads in the field order JMultiCard's OptionalDetails expects. */
final class Dg13Fixtures {
    static final String SEPARATOR = "\u0000\u0000";
    static final String[] SYNTHETIC_FIELDS = {
        "HEADER", "EJEMPLO", "PRUEBA", "ANA", "00000000-T", "01 01 1990", "ESP",
        "01 01 2030", "AAA000000", "F", "MADRID", "MADRID",
        "PERSONA UNO / PERSONA DOS", "CALLE DE EJEMPLO 1", "MADRID", "MADRID", "ESPANA"
    };

    private Dg13Fixtures() {}

    static byte[] encodedDg13(final String... fields) {
        final byte[] value = String.join(SEPARATOR, fields).getBytes(StandardCharsets.UTF_8);
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

    /** Replaces the last field so the joined payload has exactly {@code total} bytes. */
    static String paddingForTotalLength(final String[] fields, final int total) {
        final String[] others = new String[fields.length - 1];
        System.arraycopy(fields, 0, others, 0, others.length);
        final int used = String.join(SEPARATOR, others).getBytes(StandardCharsets.UTF_8).length
            + SEPARATOR.length();
        return "X".repeat(total - used);
    }
}
