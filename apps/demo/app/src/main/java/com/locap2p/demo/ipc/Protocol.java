package com.locap2p.demo.ipc;

import java.io.IOException;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import com.locap2p.demo.model.Position;
import com.locap2p.demo.model.PositionEvent;

public final class Protocol {
    private static final ObjectMapper MAPPER = new ObjectMapper()
            .disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

    private Protocol() {
    }

    public sealed interface ServerMessage {
        record Ready(String nodeId) implements ServerMessage {
        }

        record EventMessage(FrameworkEvent event) implements ServerMessage {
        }

        record PeerJoined(String nodeId) implements ServerMessage {
        }

        record PeerLeft(String nodeId) implements ServerMessage {
        }

        record Error(String message) implements ServerMessage {
        }
    }

    public record FrameworkEvent(
            String id,
            String author,
            long seq,
            @JsonProperty("timestamp_nanos") long timestampNanos,
            JsonNode payload) {
    }

    public static ServerMessage parseServerMessage(String line) throws IOException {
        JsonNode root = MAPPER.readTree(line);
        String type = root.path("type").asText();

        return switch (type) {
            case "ready" -> new ServerMessage.Ready(root.path("node_id").asText());
            case "event" -> new ServerMessage.EventMessage(
                    MAPPER.treeToValue(root.path("event"), FrameworkEvent.class));
            case "peer_joined" -> new ServerMessage.PeerJoined(root.path("node_id").asText());
            case "peer_left" -> new ServerMessage.PeerLeft(root.path("node_id").asText());
            case "error" -> new ServerMessage.Error(root.path("message").asText());
            default -> throw new IllegalArgumentException("unknown frame type: " + type);
        };
    }

    public static String publish(Object payload) {
        ObjectNode frame = MAPPER.createObjectNode();
        frame.put("type", "publish");
        frame.set("payload", MAPPER.valueToTree(payload));

        return frame.toString();
    }

    public static PositionEvent parsePositionEvent(FrameworkEvent event)
            throws IOException {
        Position position = MAPPER.treeToValue(event.payload(), Position.class);
        return new PositionEvent(event.author(), position, event.seq());
    }
}
