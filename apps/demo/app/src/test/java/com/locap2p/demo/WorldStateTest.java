package com.locap2p.demo;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

import com.locap2p.demo.model.Direction;
import com.locap2p.demo.model.Position;

class WorldStateTest {
    @Test
    void tracksLocalPosition() {
        WorldState world = new WorldState("local");

        world.moveLocal(Direction.RIGHT);
        Position position = world.moveLocal(Direction.UP);

        assertEquals(new Position(1, 1), position);
        assertEquals(position, world.localPosition());
    }

    @Test
    void appliesOnlyNewerRemoteEvents() {
        WorldState world = new WorldState("local");

        assertTrue(world.applyRemote("peer", 2, new Position(2, 0)));
        assertFalse(world.applyRemote("peer", 1, new Position(1, 0)));
        assertFalse(world.applyRemote("local", 3, new Position(9, 9)));
        assertEquals(new Position(2, 0), world.peers().get("peer").position());
    }
}
