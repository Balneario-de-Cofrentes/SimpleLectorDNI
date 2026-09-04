package es.cofrentes.simplelectordni;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.Logger;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;

public final class Worker {
    private static final int PROTOCOL_VERSION = 1;
    private static final ObjectMapper JSON = new ObjectMapper();

    private Worker() {}

    public static void main(final String[] args) throws Exception {
        silenceLibraryLogs();
        final BufferedReader input = new BufferedReader(
            new InputStreamReader(System.in, StandardCharsets.UTF_8)
        );
        System.out.println(handle(input.readLine(), new Dg13Reader()));
    }

    static String handle(final String line, final DniReader reader) {
        try {
            final EngineRequest request = parseRequest(line);
            final DniReadResult result = reader.read(request.reader_name());
            return serialize(new SuccessResponse(PROTOCOL_VERSION, "ok", result.document(), result.integrity()));
        }
        catch (final InvalidRequestException e) {
            return failure("INVALID_REQUEST", "invalid engine request", false);
        }
        catch (final DniReadException e) {
            return failure(e.code(), publicMessage(e.code()), e.retryable());
        }
        catch (final Exception e) {
            return failure("INTERNAL_ERROR", "unexpected engine error", true);
        }
    }

    private static EngineRequest parseRequest(final String line) throws InvalidRequestException {
        try {
            final EngineRequest request = JSON.readValue(line, EngineRequest.class);
            if (request.protocol() != PROTOCOL_VERSION || !"read".equals(request.command())) {
                throw new InvalidRequestException();
            }
            if (
                request.reader_name() == null ||
                request.reader_name().isBlank() ||
                request.reader_name().length() > 512
            ) {
                throw new InvalidRequestException();
            }
            return request;
        }
        catch (final JsonProcessingException | NullPointerException e) {
            throw new InvalidRequestException();
        }
    }

    private static String failure(final String code, final String message, final boolean retryable) {
        return serialize(
            new FailureResponse(
                PROTOCOL_VERSION,
                "error",
                new SafeError(safeCode(code), message, retryable)
            )
        );
    }

    private static String safeCode(final String code) {
        if (code != null && code.matches("[A-Z_]{1,64}")) {
            return code;
        }
        return "INTERNAL_ERROR";
    }

    private static String publicMessage(final String code) {
        return switch (code) {
            case "UNSUPPORTED_CARD" -> "unsupported smart card";
            case "READER_NOT_FOUND" -> "configured reader is unavailable";
            case "INTEGRITY_ERROR" -> "DNIe integrity verification failed";
            default -> "DNIe read failed";
        };
    }

    private static String serialize(final Object value) {
        try {
            return JSON.writeValueAsString(value);
        }
        catch (final JsonProcessingException e) {
            return "{\"protocol\":1,\"status\":\"error\",\"error\":{"
                + "\"code\":\"SERIALIZATION_ERROR\","
                + "\"message\":\"engine response failed\",\"retryable\":false}}";
        }
    }

    private static void silenceLibraryLogs() {
        final Logger root = Logger.getLogger("");
        root.setLevel(Level.OFF);
        for (final Handler handler : root.getHandlers()) {
            handler.setLevel(Level.OFF);
        }
    }

    private record EngineRequest(int protocol, String command, String reader_name) {}

    private record SuccessResponse(
        int protocol,
        String status,
        DocumentData document,
        IntegrityResult integrity
    ) {}

    private record FailureResponse(int protocol, String status, SafeError error) {}

    private record SafeError(String code, String message, boolean retryable) {}

    private static final class InvalidRequestException extends Exception {
        private static final long serialVersionUID = 1L;
    }
}
