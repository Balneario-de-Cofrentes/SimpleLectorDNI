package es.cofrentes.simplelectordni;

import com.fasterxml.jackson.annotation.JsonValue;

enum VerificationStatus {
    VERIFIED("verified"),
    UNVERIFIED("unverified");

    private final String wireValue;

    VerificationStatus(final String wireValue) {
        this.wireValue = wireValue;
    }

    @JsonValue
    String wireValue() {
        return wireValue;
    }
}
