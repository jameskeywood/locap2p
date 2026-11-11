package com.locap2p.demo;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

import com.locap2p.demo.model.Position;
import com.locap2p.demo.model.PositionEvent;

class ApplicationStateTest {
    private static final String LOCAL_NODE = "local";

    @Test
    void startsWithLocalPositionEvent() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));

        assertEquals(
                new PositionEvent(LOCAL_NODE, new Position(0, 0), 0),
                state.local());
    }

    @Test
    void appliesNewerLocalEvents() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));

        assertTrue(state.apply(
                new PositionEvent(LOCAL_NODE, new Position(1, 1), 1)));
        assertFalse(state.apply(
                new PositionEvent(LOCAL_NODE, new Position(0, 0), 0)));
        assertEquals(new Position(1, 1), state.local().position());
    }

    @Test
    void tracksRemotePlayersIndependently() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));

        assertTrue(state.apply(
                new PositionEvent("peer", new Position(2, 0), 2)));
        assertFalse(state.apply(
                new PositionEvent("peer", new Position(1, 0), 1)));
        assertEquals(new Position(2, 0), state.positions().get("peer").position());
        assertEquals(new Position(0, 0), state.local().position());
    }
}
