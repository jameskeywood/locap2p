package com.locap2p.demo.ipc;

import java.io.IOException;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import com.locap2p.demo.model.config.FrameworkConfig;
import com.locap2p.demo.model.ipc.FrameworkEvent;
import com.locap2p.demo.model.ipc.FrameworkMessage;
import com.locap2p.demo.model.player.Position;
import com.locap2p.demo.model.player.PositionEvent;

/**
 * Encodes and decodes IPC frames exchanged with the framework host.
 */
public final class Protocol {

    private static final ObjectMapper MAPPER = new ObjectMapper()
            .disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

    public static FrameworkMessage parseFrameworkMessage(String line) throws IOException {
        JsonNode root = MAPPER.readTree(line);
        String type = root.path("type").asText();

        return switch (type) {
            case "ready" -> new FrameworkMessage.Ready(root.path("node_id").asText());

            case "event" -> new FrameworkMessage.EventMessage(
                MAPPER.treeToValue(root.path("event"),
                FrameworkEvent.class)
            );

            case "peer_joined" -> new FrameworkMessage.PeerJoined(root.path("node_id").asText());

            case "peer_left" -> new FrameworkMessage.PeerLeft(root.path("node_id").asText());

            case "error" -> new FrameworkMessage.Error(root.path("message").asText());

            default -> throw new IllegalArgumentException("unknown frame type: " + type);
        };
    }

    public static PositionEvent parsePositionEvent(FrameworkEvent event) throws IOException {
        Position position = MAPPER.treeToValue(event.payload(), Position.class);

        return new PositionEvent(event.author(), position, event.seq());
    }

    public static String configure(FrameworkConfig config) throws IOException {
        ObjectNode frame = MAPPER.createObjectNode();
        frame.put("type", "configure");
        frame.set("config", MAPPER.readTree(config.toJson()));

        return frame.toString();
    }

    public static String publish(Object payload) {
        ObjectNode frame = MAPPER.createObjectNode();
        frame.put("type", "publish");
        frame.set("payload", MAPPER.valueToTree(payload));

        return frame.toString();
    }

}
