package com.locap2p.demo.ipc;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import java.io.IOException;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import org.junit.jupiter.api.Test;

import com.locap2p.demo.model.ipc.FrameworkMessage;
import com.locap2p.demo.model.player.Position;
import com.locap2p.demo.model.player.PositionEvent;

class ProtocolTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    void parsesEventMessage() throws IOException {
        String line = """
                {
                  "type": "event",
                  "event": {
                    "id": "e1",
                    "author": "peer",
                    "seq": 2,
                    "timestamp_nanos": 10,
                    "payload": {"x": 3, "y": 4}
                  }
                }
                """;

        FrameworkMessage message = Protocol.parseFrameworkMessage(line);

        FrameworkMessage.EventMessage event = assertInstanceOf(
            FrameworkMessage.EventMessage.class,
            message
        );

        assertEquals("peer", event.event().author());
        assertEquals(2, event.event().seq());
        assertEquals(10, event.event().timestampNanos());
        assertEquals(
                new PositionEvent("peer", new Position(3, 4), 2),
                Protocol.parsePositionEvent(event.event()));
    }

    @Test
    void serializesPublishFrame() throws IOException {
        JsonNode frame = MAPPER.readTree(Protocol.publish(new Position(1, 2)));

        assertEquals("publish", frame.path("type").asText());
        assertEquals(1, frame.path("payload").path("x").asInt());
        assertEquals(2, frame.path("payload").path("y").asInt());
    }
}
