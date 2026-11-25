package com.locap2p.demo.ipc;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.net.StandardProtocolFamily;
import java.net.UnixDomainSocketAddress;
import java.nio.channels.Channel;
import java.nio.channels.Channels;
import java.nio.channels.SelectionKey;
import java.nio.channels.Selector;
import java.nio.channels.ServerSocketChannel;
import java.nio.channels.SocketChannel;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.PosixFilePermissions;
import java.time.Duration;
import java.util.Objects;
import java.util.concurrent.TimeUnit;

/**
 * A Unix domain socket connection to the framework host.
 */
public final class IpcSocket implements AutoCloseable {
    private static final String DIRECTORY_PREFIX = "locap2p-";
    private static final String DIRECTORY_PERMISSIONS = "rwx------";
    private static final String SOCKET_FILE_NAME = "ipc.sock";
    private static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);

    private final Path socketDir;
    private final Path socketPath;
    private ServerSocketChannel listener;
    private SocketChannel channel;
    private BufferedReader reader;
    private BufferedWriter writer;

    private IpcSocket(
            Path socketDir,
            Path socketPath,
            ServerSocketChannel listener) {

        this.socketDir = socketDir;
        this.socketPath = socketPath;
        this.listener = listener;
    }

    public static IpcSocket listen() throws IOException {
        Path socketDir = Files.createTempDirectory(
                DIRECTORY_PREFIX,
                PosixFilePermissions.asFileAttribute(
                        PosixFilePermissions.fromString(DIRECTORY_PERMISSIONS)));
        Path socketPath = socketDir.resolve(SOCKET_FILE_NAME);

        ServerSocketChannel listener = null;
        try {
            listener = ServerSocketChannel.open(StandardProtocolFamily.UNIX);
            listener.bind(UnixDomainSocketAddress.of(socketPath));
            return new IpcSocket(socketDir, socketPath, listener);
        } catch (IOException | RuntimeException error) {
            closeQuietly(listener);
            deleteQuietly(socketPath);
            deleteQuietly(socketDir);
            throw error;
        }
    }

    public Path path() {
        return socketPath;
    }

    public void accept(Process process) throws IOException {
        Objects.requireNonNull(process, "process");

        if (listener == null) {
            throw new IllegalStateException("socket is not listening");
        }

        try {
            listener.configureBlocking(false);

            try (Selector selector = Selector.open()) {
                listener.register(selector, SelectionKey.OP_ACCEPT);
                long deadline = System.nanoTime() + CONNECT_TIMEOUT.toNanos();

                while (true) {
                    ensureProcessIsAlive(process);

                    long remainingNanos = deadline - System.nanoTime();
                    if (remainingNanos <= 0) {
                        throw new IOException(
                                "timed out waiting for the framework host");
                    }

                    long remainingMillis = Math.max(
                            1,
                            TimeUnit.NANOSECONDS.toMillis(remainingNanos));
                    if (selector.select(remainingMillis) == 0) {
                        continue;
                    }

                    SocketChannel accepted = listener.accept();
                    if (accepted != null) {
                        configureAcceptedChannel(accepted);
                        return;
                    }
                }
            }
        } finally {
            closeListener();
        }
    }

    public String readLine() throws IOException {
        return requireReader().readLine();
    }

    public void writeLine(String line) throws IOException {
        BufferedWriter currentWriter = requireWriter();
        currentWriter.write(line);
        currentWriter.newLine();
        currentWriter.flush();
    }

    public void shutdownOutput() throws IOException {
        if (writer != null) {
            writer.flush();
        }
        if (channel != null) {
            channel.shutdownOutput();
        }
    }

    @Override
    public void close() {
        closeQuietly(channel);
        channel = null;
        closeListener();
        deleteQuietly(socketPath);
        deleteQuietly(socketDir);
    }

    private void configureAcceptedChannel(SocketChannel accepted) throws IOException {
        try {
            accepted.configureBlocking(true);

            reader = new BufferedReader(
                    Channels.newReader(accepted, StandardCharsets.UTF_8));
            writer = new BufferedWriter(
                    Channels.newWriter(accepted, StandardCharsets.UTF_8));
            channel = accepted;
        } catch (IOException | RuntimeException error) {
            closeQuietly(accepted);
            throw error;
        }
    }

    private static void ensureProcessIsAlive(Process process) throws IOException {
        if (!process.isAlive()) {
            throw new IOException(
                    "framework host exited before connecting: "
                            + process.exitValue());
        }
    }

    private BufferedReader requireReader() {
        if (reader == null) {
            throw new IllegalStateException("socket is not connected");
        }
        return reader;
    }

    private BufferedWriter requireWriter() {
        if (writer == null) {
            throw new IllegalStateException("socket is not connected");
        }
        return writer;
    }

    private void closeListener() {
        if (listener != null) {
            closeQuietly(listener);
            listener = null;
        }
    }

    private static void closeQuietly(Channel channel) {
        if (channel == null) {
            return;
        }

        try {
            channel.close();
        } catch (IOException ignored) {
        }
    }

    private static void deleteQuietly(Path path) {
        if (path == null) {
            return;
        }

        try {
            Files.deleteIfExists(path);
        } catch (IOException ignored) {
        }
    }
}
