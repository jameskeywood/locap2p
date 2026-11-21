package com.locap2p.demo;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;

import com.locap2p.demo.ipc.FrameworkClient;
import com.locap2p.demo.ipc.Protocol;
import com.locap2p.demo.model.config.FrameworkConfig;
import com.locap2p.demo.model.ipc.FrameworkEvent;
import com.locap2p.demo.model.ipc.FrameworkMessage;
import com.locap2p.demo.model.player.Direction;
import com.locap2p.demo.model.player.Position;
import com.locap2p.demo.model.player.PositionEvent;

public final class Main {
    private Main() {
    }

    public static void main(String[] args) throws Exception {
        Arguments arguments = parseArguments(args);
        if (arguments == null) {
            printUsage();
            System.exit(2);
            return;
        }

        Path hostBinary = arguments.hostBinary().toAbsolutePath().normalize();
        if (!Files.isExecutable(hostBinary)) {
            System.err.println("framework host not found at " + hostBinary);
            System.err.println("build it with: cargo build -p locap2p-host");
            System.exit(1);
            return;
        }

        FrameworkConfig config;
        try {
            config = FrameworkConfig.load(arguments.config());
        } catch (IOException | IllegalArgumentException error) {
            System.err.println("failed to load configuration: " + error.getMessage());
            System.exit(1);
            return;
        }

        try (FrameworkClient client = FrameworkClient.start(hostBinary, config)) {
            ApplicationState state = new ApplicationState(
                    client.nodeId(),
                    new Position(0, 0));
            client.listen(message -> handleServerMessage(state, message));

            System.out.println("Node: " + client.nodeId());
            System.out.println("Session: " + config.application().session());
            System.out.println("Commands: up, down, left, right, pos, quit");

            readCommands(client, state);
        }
    }

    private static Arguments parseArguments(String[] args) {
        Path hostBinary = null;
        Path config = null;

        for (int index = 0; index < args.length; index++) {
            switch (args[index]) {
                case "--host-bin" -> {
                    if (index + 1 >= args.length) {
                        return null;
                    }
                    hostBinary = Path.of(args[++index]);
                }
                case "--config" -> {
                    if (index + 1 >= args.length) {
                        return null;
                    }
                    config = Path.of(args[++index]);
                }
                default -> {
                    return null;
                }
            }
        }

        if (hostBinary == null || config == null) {
            return null;
        }
        return new Arguments(hostBinary, config);
    }

    private static void printUsage() {
        System.err.println(
                "usage: locap2p-demo --host-bin <path> --config <path>");
    }

    private record Arguments(Path hostBinary, Path config) {
    }

    private static void readCommands(
        FrameworkClient client,
        ApplicationState state) throws IOException {

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
                case "pos" -> printLocal(state.local());
                case "up" -> move(client, state, Direction.UP);
                case "down" -> move(client, state, Direction.DOWN);
                case "left" -> move(client, state, Direction.LEFT);
                case "right" -> move(client, state, Direction.RIGHT);
                default -> System.out.println("unknown command: " + command);
            }
        }
    }

    private static void move(
        FrameworkClient client,
        ApplicationState state,
        Direction direction) throws IOException {

        Position next = state.local().position().movedBy(direction);
        client.publish(next);
    }

    private static void handleServerMessage(ApplicationState state, FrameworkMessage message) {
        switch (message) {
            case FrameworkMessage.EventMessage event ->
                    handleEvent(state, event.event());
            case FrameworkMessage.PeerJoined joined ->
                    System.out.printf("peer joined: %s%n", joined.nodeId());
            case FrameworkMessage.PeerLeft left ->
                    System.out.printf("peer left: %s%n", left.nodeId());
            case FrameworkMessage.Error error ->
                    System.err.printf("framework error: %s%n", error.message());
            case FrameworkMessage.Ready ignored -> {
            }
        }
    }

    private static void handleEvent(ApplicationState state, FrameworkEvent frameworkEvent) {
        try {
            PositionEvent event = Protocol.parsePositionEvent(frameworkEvent);
            if (!state.apply(event)) {
                return;
            }

            if (state.isLocal(event.author())) {
                printLocal(event);
            } else {
                System.out.printf(
                        "peer %s: (%d, %d)%n",
                        event.author(),
                        event.position().x(),
                        event.position().y());
            }
        } catch (IOException | IllegalArgumentException error) {
            System.err.printf(
                    "ignored malformed event from %s: %s%n",
                    frameworkEvent.author(),
                    error.getMessage());
        }
    }

    private static void printLocal(PositionEvent event) {
        System.out.printf(
                "you: (%d, %d)%n",
                event.position().x(),
                event.position().y());
    }
}
