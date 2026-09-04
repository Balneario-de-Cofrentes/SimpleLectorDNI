package es.cofrentes.simplelectordni;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;

import org.junit.jupiter.api.Test;

final class WorkerOutputTest {

    @Test
    void responseBytesAreUtf8RegardlessOfPlatformCharset() throws Exception {
        final ByteArrayOutputStream output = new ByteArrayOutputStream();

        Worker.write(output, "{\"nombre\":\"Ñ\"}");

        final byte[] expected = "{\"nombre\":\"Ñ\"}\n".getBytes(StandardCharsets.UTF_8);
        assertArrayEquals(expected, output.toByteArray());
        assertArrayEquals(new byte[] {(byte) 0xC3, (byte) 0x91}, new byte[] {expected[11], expected[12]});
    }
}
