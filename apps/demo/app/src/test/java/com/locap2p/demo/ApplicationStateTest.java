package com.locap2p.demo;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;

import org.junit.jupiter.api.Test;

import com.locap2p.demo.model.player.Player;
import com.locap2p.demo.model.player.Position;
import com.locap2p.demo.model.player.PositionEvent;

class ApplicationStateTest {
    private static final String LOCAL_NODE = "local";

    @Test
    void startsWithLocalPlayer() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));

        Player local = state.local();

        assertEquals(LOCAL_NODE, local.nodeId());
        assertEquals(new Position(0, 0), local.position());
        assertEquals(0, local.seq());
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
        assertEquals(
                new Position(2, 0),
                state.peer("peer").orElseThrow().position());
        assertEquals(new Position(0, 0), state.local().position());
    }

    @Test
    void peersExposeTheirOwnState() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));
        state.apply(new PositionEvent("peer", new Position(3, 4), 7));

        List<Player> peers = state.peers();

        assertEquals(1, peers.size());

        Player peer = peers.get(0);
        assertEquals("peer", peer.nodeId());
        assertEquals(new Position(3, 4), peer.position());
        assertEquals(7, peer.seq());
        assertTrue(peer.ageMillis(System.nanoTime()) >= 0);
    }

    @Test
    void peersAreOrderedByNodeId() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));
        state.apply(new PositionEvent("peer-b", new Position(0, 0), 1));
        state.apply(new PositionEvent("peer-a", new Position(0, 0), 1));

        assertEquals(
                List.of("peer-a", "peer-b"),
                state.peers().stream().map(Player::nodeId).toList());
    }

    @Test
    void joinedPeerWithoutPositionIsOnlyAMember() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));
        state.peerJoined("peer");

        assertTrue(state.peers().isEmpty());
        assertTrue(state.peer("peer").isEmpty());
        assertEquals(1, state.memberCount());
    }

    @Test
    void leavingPeerIsRemovedFromState() {
        ApplicationState state = new ApplicationState(LOCAL_NODE, new Position(0, 0));
        state.peerJoined("peer");
        state.apply(new PositionEvent("peer", new Position(1, 1), 1));

        assertEquals(1, state.peers().size());

        state.peerLeft("peer");

        assertTrue(state.peers().isEmpty());
        assertTrue(state.peer("peer").isEmpty());
        assertEquals(0, state.memberCount());
    }
}
