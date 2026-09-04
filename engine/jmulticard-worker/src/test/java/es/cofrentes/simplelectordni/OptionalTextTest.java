package es.cofrentes.simplelectordni;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

final class OptionalTextTest {

    @Test
    void optionalEnrichmentFailureDoesNotDiscardDg13() {
        assertEquals("", Dg13Reader.optionalText(() -> {
            throw new IllegalStateException("optional file unavailable");
        }));
        assertEquals("4.0", Dg13Reader.optionalText(() -> " 4.0 "));
    }
}
