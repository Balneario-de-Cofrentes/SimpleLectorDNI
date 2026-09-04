package es.cofrentes.simplelectordni;

import java.nio.charset.StandardCharsets;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.time.format.ResolverStyle;
import java.util.Arrays;
import java.util.Locale;
import java.util.regex.Pattern;

/**
 * Splits the DG13 value into its text fields after removing the DER header.
 *
 * JMultiCard's {@code OptionalDetails} splits the raw bytes, header included, and
 * returns the current date when a date field is missing. This parser keeps the
 * same field order but refuses both behaviours.
 */
final class Dg13TextFields {
    static final int ID_NUMBER = 4;
    static final int BIRTH_DATE = 5;
    static final int EXPIRY_DATE = 7;

    private static final byte DG13_TAG = 0x6D;
    private static final Pattern SEPARATOR = Pattern.compile("\\p{Cc}{2}");
    private static final DateTimeFormatter DATE = DateTimeFormatter
        .ofPattern("dd MM uuuu", Locale.ROOT)
        .withResolverStyle(ResolverStyle.STRICT);

    private final String[] values;

    private Dg13TextFields(final String[] values) {
        this.values = values;
    }

    static Dg13TextFields from(final byte[] bytes) {
        final String text = new String(derValue(bytes), StandardCharsets.ISO_8859_1);
        return new Dg13TextFields(SEPARATOR.split(text, -1));
    }

    private static byte[] derValue(final byte[] bytes) {
        if (bytes == null || bytes.length < 2 || bytes[0] != DG13_TAG) {
            return new byte[0];
        }
        final int firstLength = Byte.toUnsignedInt(bytes[1]);
        final int lengthOctets = firstLength < 128 ? 0 : firstLength & 0x7F;
        if (firstLength == 128 || lengthOctets > 3 || bytes.length < 2 + lengthOctets) {
            return new byte[0];
        }
        final int offset = 2 + lengthOctets;
        final int length = lengthOctets == 0
            ? firstLength
            : decodeLength(bytes, lengthOctets);
        return length == bytes.length - offset
            ? Arrays.copyOfRange(bytes, offset, bytes.length)
            : new byte[0];
    }

    private static int decodeLength(final byte[] bytes, final int lengthOctets) {
        int length = 0;
        for (int index = 0; index < lengthOctets; index++) {
            length = (length << 8) | Byte.toUnsignedInt(bytes[2 + index]);
        }
        return length;
    }

    String textAt(final int index) {
        return index >= 0 && index < values.length ? values[index].trim() : "";
    }

    String isoDateAt(final int index) {
        final String value = textAt(index);
        if (value.isEmpty()) {
            return "";
        }
        try {
            return LocalDate.parse(value, DATE).toString();
        }
        catch (final DateTimeParseException e) {
            return "";
        }
    }
}
