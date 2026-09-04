package es.cofrentes.simplelectordni;

final class DniReadException extends Exception {
    private final String code;
    private final boolean retryable;

    DniReadException(
        final String code,
        final String safeMessage,
        final boolean retryable,
        final Throwable cause
    ) {
        super(safeMessage, cause);
        this.code = code;
        this.retryable = retryable;
    }

    String code() {
        return code;
    }

    boolean retryable() {
        return retryable;
    }
}
