package es.cofrentes.simplelectordni;

final class DniReadException extends Exception {
    private final DniErrorCode error;

    DniReadException(final DniErrorCode error) {
        this(error, null);
    }

    DniReadException(final DniErrorCode error, final Throwable cause) {
        super(error.publicMessage(), cause);
        this.error = error;
    }

    DniErrorCode error() {
        return error;
    }
}
