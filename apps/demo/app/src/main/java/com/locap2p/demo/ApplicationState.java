package com.locap2p.demo;

import java.util.HashMap;
import java.util.Map;
import java.util.Objects;

import com.locap2p.demo.model.Position;
import com.locap2p.demo.model.PositionEvent;

public final class ApplicationState {
    private static final long INITIAL_SEQ = 0;

    private final String localNodeId;
    private final Map<String, PositionEvent> positions = new HashMap<>();

    public ApplicationState(String localNodeId, Position initialPosition) {
        this.localNodeId = Objects.requireNonNull(localNodeId, "localNodeId");

        PositionEvent initial = new PositionEvent(
                localNodeId,
                Objects.requireNonNull(initialPosition, "initialPosition"),
                INITIAL_SEQ);

        positions.put(localNodeId, initial);
    }

    public synchronized PositionEvent local() {
        return positions.get(localNodeId);
    }

    public synchronized boolean isLocal(String author) {
        return localNodeId.equals(author);
    }

    public synchronized boolean apply(PositionEvent event) {
        PositionEvent current = positions.get(event.author());
        if (current != null && event.seq() <= current.seq()) {
            return false;
        }

        positions.put(event.author(), event);
        return true;
    }

    public synchronized Map<String, PositionEvent> positions() {
        return Map.copyOf(positions);
    }
}
