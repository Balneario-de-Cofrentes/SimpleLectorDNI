package es.cofrentes.simplelectordni;

enum DniErrorCode {
    CARD_READ_FAILED("DNIe read failed", true),
    UNSUPPORTED_CARD("unsupported smart card", false),
    READER_NOT_FOUND("configured reader is unavailable", true),
    INTEGRITY_ERROR("DNIe integrity verification failed", false);

    private final String publicMessage;
    private final boolean retryable;

    DniErrorCode(final String publicMessage, final boolean retryable) {
        this.publicMessage = publicMessage;
        this.retryable = retryable;
    }

    String publicMessage() {
        return publicMessage;
    }

    boolean retryable() {
        return retryable;
    }
}
