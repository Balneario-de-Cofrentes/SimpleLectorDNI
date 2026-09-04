package es.cofrentes.simplelectordni;

import java.nio.charset.StandardCharsets;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.time.format.ResolverStyle;
import java.util.Locale;
import java.util.regex.Pattern;

final class Dg13TextFields {
    private static final Pattern SEPARATOR = Pattern.compile("\\p{Cc}{2}");
    private static final DateTimeFormatter DATE = DateTimeFormatter
        .ofPattern("dd MM uuuu", Locale.ROOT)
        .withResolverStyle(ResolverStyle.STRICT);

    private final String[] values;

    private Dg13TextFields(final String[] values) {
        this.values = values;
    }

    static Dg13TextFields from(final byte[] bytes) {
        final String text = new String(bytes, StandardCharsets.ISO_8859_1);
        return new Dg13TextFields(SEPARATOR.split(text, -1));
    }

    String isoDateAt(final int index) {
        if (index < 0 || index >= values.length || values[index].isBlank()) {
            return "";
        }
        try {
            return LocalDate.parse(values[index].trim(), DATE).toString();
        }
        catch (final DateTimeParseException e) {
            return "";
        }
    }
}
