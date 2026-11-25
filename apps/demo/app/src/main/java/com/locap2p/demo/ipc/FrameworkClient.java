package com.locap2p.demo.ipc;

import java.io.IOException;
import java.nio.file.Path;
import java.util.Objects;
import java.util.concurrent.TimeUnit;
import java.util.function.Consumer;

import com.locap2p.demo.model.config.FrameworkConfig;
import com.locap2p.demo.model.ipc.FrameworkMessage;

public final class FrameworkClient implements AutoCloseable {
    private static final long SHUTDOWN_TIMEOUT_SECONDS = 2;

    private final Process process;
    private final IpcSocket socket;
    private final String nodeId;

    private FrameworkClient(Process process, IpcSocket socket, String nodeId) {
        this.process = Objects.requireNonNull(process, "process");
        this.socket = Objects.requireNonNull(socket, "socket");
        this.nodeId = Objects.requireNonNull(nodeId, "nodeId");
    }

    public static FrameworkClient start(
        Path hostBinary,
        FrameworkConfig config) throws IOException {

        IpcSocket socket = IpcSocket.listen();
        Process process = null;

        try {
            process = new ProcessBuilder(
                hostBinary.toString(),
                "--ipc-socket", socket.path().toString())
                .redirectError(ProcessBuilder.Redirect.INHERIT)
                .redirectOutput(ProcessBuilder.Redirect.DISCARD)
                .start();
            process.getOutputStream().close();

            socket.accept(process);
            socket.writeLine(Protocol.configure(config));

            String readyLine = socket.readLine();
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

            FrameworkClient client = new FrameworkClient(process, socket, ready.nodeId());
            process = null;
            return client;
        }
        catch (IOException | RuntimeException error) {
            if (process != null) {
                process.destroyForcibly();
            }
            socket.close();
            throw error;
        }
    }

    public String nodeId() {
        return nodeId;
    }

    public synchronized void listen(Consumer<FrameworkMessage> listener) {
        Thread.ofVirtual()
                .name("locap2p-ipc-reader")
                .start(() -> readFrames(listener));
    }

    public synchronized void publish(Object payload) throws IOException {
        socket.writeLine(Protocol.publish(payload));
    }

    @Override
    public void close() {
        try {
            socket.shutdownOutput();
        } catch (IOException ignored) {
        }

        try {
            if (!process.waitFor(SHUTDOWN_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
                process.destroyForcibly();
            }
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            process.destroyForcibly();
        } finally {
            socket.close();
        }
    }

    private void readFrames(Consumer<FrameworkMessage> listener) {
        try {
            String line;
            while ((line = socket.readLine()) != null) {
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
