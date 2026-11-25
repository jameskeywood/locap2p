package com.locap2p.demo.model.player;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class PlayerTest {
    private static final String NODE = "node";

    @Test
    void acceptsOnlyNewerEvents() {
        Player player = new Player(NODE, new Position(0, 0), 3, 0);

        assertTrue(player.accepts(new PositionEvent(NODE, new Position(1, 1), 4)));
        assertFalse(player.accepts(new PositionEvent(NODE, new Position(1, 1), 3)));
        assertFalse(player.accepts(new PositionEvent(NODE, new Position(1, 1), 2)));
    }

    @Test
    void updateReplacesPositionSequenceAndObservationTime() {
        Player player = new Player(NODE, new Position(0, 0), 1, 10);

        Player updated = player.updated(
                new PositionEvent(NODE, new Position(2, 3), 2),
                20);

        assertEquals(NODE, updated.nodeId());
        assertEquals(new Position(2, 3), updated.position());
        assertEquals(2, updated.seq());
        assertEquals(20, updated.observedAtNanos());
    }

    @Test
    void ageIsMeasuredFromObservationTime() {
        Player player = new Player(NODE, new Position(0, 0), 1, 1_000_000_000L);

        assertEquals(500, player.ageMillis(1_500_000_000L));
    }

    @Test
    void ageNeverGoesNegative() {
        Player player = new Player(NODE, new Position(0, 0), 1, 1_000_000_000L);

        assertEquals(0, player.ageMillis(0));
    }
}
