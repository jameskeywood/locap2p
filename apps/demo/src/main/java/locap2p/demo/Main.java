package locap2p.demo;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;

public final class Main {
    private Main() {
    }

    public static void main(String[] args) throws Exception {
        Path hostBinary = resolveHostBinary();
        if (!Files.isExecutable(hostBinary)) {
            System.err.println("framework host not found at " + hostBinary);
            System.err.println("build it with: cargo build -p locap2p-host");
            System.exit(1);
            return;
        }

        try (FrameworkClient client = FrameworkClient.start(hostBinary)) {
            WorldState world = new WorldState(client.nodeId());
            client.listen(message -> handleServerMessage(world, message));

            System.out.println("Node: " + client.nodeId());
            System.out.println("Commands: up, down, left, right, pos, quit");

            readCommands(client, world);
        }
    }

    private static Path resolveHostBinary() {
        String configured = System.getProperty(
                "locap2p.host.bin",
                System.getenv("LOCAP2P_HOST_BIN"));

        if (configured != null && !configured.isBlank()) {
            return Path.of(configured).toAbsolutePath().normalize();
        }

        return Path.of("..", "..", "target", "debug", "locap2p-host")
                .toAbsolutePath()
                .normalize();
    }

    private static void readCommands(FrameworkClient client, WorldState world) throws IOException {
        BufferedReader input = new BufferedReader(
                new InputStreamReader(System.in, StandardCharsets.UTF_8));

        String line;
        while ((line = input.readLine()) != null) {
            String command = line.trim().toLowerCase(Locale.ROOT);
            if (command.isEmpty()) {
                continue;
            }

            switch (command) {
                case "quit" -> {
                    return;
                }
                case "pos" -> printLocal(world.localPosition());
                case "up" -> move(client, world, Direction.UP);
                case "down" -> move(client, world, Direction.DOWN);
                case "left" -> move(client, world, Direction.LEFT);
                case "right" -> move(client, world, Direction.RIGHT);
                default -> System.out.println("unknown command: " + command);
            }
        }
    }

    private static void move(FrameworkClient client, WorldState world, Direction direction)
            throws IOException {
        Position position = world.moveLocal(direction);
        client.publish(position);
        printLocal(position);
    }

    private static void handleServerMessage(WorldState world, Protocol.ServerMessage message) {
        switch (message) {
            case Protocol.ServerMessage.RemoteEvent remote -> handleEvent(world, remote.event());
            case Protocol.ServerMessage.PeerJoined joined ->
                    System.out.printf("peer joined: %s%n", joined.nodeId());
            case Protocol.ServerMessage.PeerLeft left ->
                    System.out.printf("peer left: %s%n", left.nodeId());
            case Protocol.ServerMessage.Error error ->
                    System.err.printf("framework error: %s%n", error.message());
            case Protocol.ServerMessage.Ready ignored -> {
            }
        }
    }

    private static void handleEvent(WorldState world, Protocol.FrameworkEvent event) {
        try {
            Position position = Protocol.parsePosition(event.payload());
            if (world.applyRemote(event.author(), event.seq(), position)) {
                System.out.printf(
                        "peer %s: (%d, %d)%n",
                        event.author(),
                        position.x(),
                        position.y());
            }
        } catch (IOException | IllegalArgumentException error) {
            System.err.printf(
                    "ignored malformed event from %s: %s%n",
                    event.author(),
                    error.getMessage());
        }
    }

    private static void printLocal(Position position) {
        System.out.printf("you: (%d, %d)%n", position.x(), position.y());
    }
}
