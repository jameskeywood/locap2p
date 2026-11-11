package locap2p.demo;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;
import java.util.function.Consumer;

public final class FrameworkClient implements AutoCloseable {
    private static final long SHUTDOWN_TIMEOUT_SECONDS = 2;

    private final Process process;
    private final BufferedWriter writer;
    private final BufferedReader reader;
    private final String nodeId;
    private Thread readerThread;

    private FrameworkClient(
            Process process,
            BufferedWriter writer,
            BufferedReader reader,
            String nodeId) {
        this.process = process;
        this.writer = writer;
        this.reader = reader;
        this.nodeId = nodeId;
    }

    public static FrameworkClient start(Path hostBinary) throws IOException {
        Process process = new ProcessBuilder(hostBinary.toString())
                .redirectError(ProcessBuilder.Redirect.INHERIT)
                .start();

        try {
            BufferedWriter writer = new BufferedWriter(
                    new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));
            BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8));

            String readyLine = reader.readLine();
            if (readyLine == null) {
                throw new IOException("framework host exited before becoming ready");
            }

            Protocol.ServerMessage first = Protocol.parseServerMessage(readyLine);
            if (!(first instanceof Protocol.ServerMessage.Ready ready)) {
                throw new IOException("expected ready frame but received: " + first);
            }

            return new FrameworkClient(process, writer, reader, ready.nodeId());
        } catch (IOException | RuntimeException error) {
            process.destroyForcibly();
            throw error;
        }
    }

    public String nodeId() {
        return nodeId;
    }

    public synchronized void listen(Consumer<Protocol.ServerMessage> listener) {
        if (readerThread != null) {
            throw new IllegalStateException("listener already started");
        }

        readerThread = Thread.ofPlatform()
                .daemon(true)
                .name("locap2p-ipc-reader")
                .start(() -> readFrames(listener));
    }

    public synchronized void publish(Object payload) throws IOException {
        writer.write(Protocol.publish(payload));
        writer.newLine();
        writer.flush();
    }

    @Override
    public void close() {
        try {
            writer.close();
        } catch (IOException ignored) {
        }

        try {
            if (!process.waitFor(SHUTDOWN_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
                process.destroyForcibly();
            }
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            process.destroyForcibly();
        }
    }

    private void readFrames(Consumer<Protocol.ServerMessage> listener) {
        try {
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.isBlank()) {
                    continue;
                }

                try {
                    listener.accept(Protocol.parseServerMessage(line));
                } catch (IOException | RuntimeException error) {
                    System.err.printf("invalid IPC frame: %s%n", error.getMessage());
                }
            }
        } catch (IOException error) {
            System.err.printf("IPC reader stopped: %s%n", error.getMessage());
        }
    }
}
