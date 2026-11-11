package com.locap2p.demo;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import java.io.IOException;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import org.junit.jupiter.api.Test;

import com.locap2p.demo.model.Position;
import com.locap2p.demo.model.PositionEvent;

class ProtocolTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    void parsesRemoteEvent() throws IOException {
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

        Protocol.ServerMessage message = Protocol.parseServerMessage(line);
        Protocol.ServerMessage.RemoteEvent remote = assertInstanceOf(
                Protocol.ServerMessage.RemoteEvent.class,
                message);

        assertEquals("peer", remote.event().author());
        assertEquals(2, remote.event().seq());
        assertEquals(10, remote.event().timestampNanos());
        assertEquals(
                new PositionEvent("peer", new Position(3, 4), 2),
                Protocol.parsePositionEvent(remote.event()));
    }

    @Test
    void serializesPublishFrame() throws IOException {
        JsonNode frame = MAPPER.readTree(Protocol.publish(new Position(1, 2)));

        assertEquals("publish", frame.path("type").asText());
        assertEquals(1, frame.path("payload").path("x").asInt());
        assertEquals(2, frame.path("payload").path("y").asInt());
    }
}
