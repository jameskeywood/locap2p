package com.locap2p.demo.model.player;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class PositionTest {
    @Test
    void movesInEachDirection() {
        Position origin = new Position(0, 0);

        assertEquals(new Position(0, 1), origin.movedBy(Direction.UP));
        assertEquals(new Position(0, -1), origin.movedBy(Direction.DOWN));
        assertEquals(new Position(-1, 0), origin.movedBy(Direction.LEFT));
        assertEquals(new Position(1, 0), origin.movedBy(Direction.RIGHT));
    }

    @Test
    void computesDistanceToAnotherPosition() {
        Position origin = new Position(0, 0);

        assertEquals(0.0, origin.distanceTo(new Position(0, 0)), 1e-9);
        assertEquals(5.0, origin.distanceTo(new Position(3, 4)), 1e-9);
        assertEquals(2.0, new Position(1, 1).distanceTo(new Position(3, 1)), 1e-9);
    }
}
