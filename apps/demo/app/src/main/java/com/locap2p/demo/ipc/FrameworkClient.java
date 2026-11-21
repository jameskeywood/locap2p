package com.locap2p.demo.ipc;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;
import java.util.function.Consumer;

import com.locap2p.demo.model.config.FrameworkConfig;
import com.locap2p.demo.model.ipc.FrameworkMessage;

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

    public static FrameworkClient start(
        Path hostBinary,
        FrameworkConfig config) throws IOException {

        Process process = new ProcessBuilder(hostBinary.toString())
                .redirectError(ProcessBuilder.Redirect.INHERIT)
                .start();

        try {
            BufferedWriter writer = new BufferedWriter(
                    new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));
            BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8));

            writer.write(Protocol.configure(config));
            writer.newLine();
            writer.flush();

            String readyLine = reader.readLine();
            if (readyLine == null) {
                throw new IOException("framework host exited before becoming ready");
            }

            FrameworkMessage first = Protocol.parseFrameworkMessage(readyLine);
            if (first instanceof FrameworkMessage.Error error) {
                throw new IOException(
                        "framework host rejected configuration: " + error.message());
            }
            if (!(first instanceof FrameworkMessage.Ready ready)) {
                throw new IOException("expected ready frame but received: " + first);
            }

            return new FrameworkClient(process, writer, reader, ready.nodeId());
        }
        catch (IOException | RuntimeException error) {
            process.destroyForcibly();
            throw error;
        }
    }

    public String nodeId() {
        return nodeId;
    }

    public synchronized void listen(Consumer<FrameworkMessage> listener) {
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

    private void readFrames(Consumer<FrameworkMessage> listener) {
        try {
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.isBlank()) {
                    continue;
                }

                try {
                    listener.accept(Protocol.parseFrameworkMessage(line));
                } catch (IOException | RuntimeException error) {
                    System.err.printf("invalid IPC frame: %s%n", error.getMessage());
                }
            }
        } catch (IOException error) {
            System.err.printf("IPC reader stopped: %s%n", error.getMessage());
        }
    }
}
