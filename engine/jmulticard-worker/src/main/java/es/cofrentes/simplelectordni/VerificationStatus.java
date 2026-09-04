package es.cofrentes.simplelectordni;

import com.fasterxml.jackson.annotation.JsonValue;

/** The only status the worker emits: a failed verification aborts the read with INTEGRITY_ERROR. */
enum VerificationStatus {
    VERIFIED("verified");

    private final String wireValue;

    VerificationStatus(final String wireValue) {
        this.wireValue = wireValue;
    }

    @JsonValue
    String wireValue() {
        return wireValue;
    }
}
