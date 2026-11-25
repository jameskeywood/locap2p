package com.locap2p.demo;

import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;

import com.locap2p.demo.model.player.Player;
import com.locap2p.demo.model.player.Position;
import com.locap2p.demo.model.player.PositionEvent;

/**
 * Holds the local player, peer players, and membership known to the application.
 */
public final class ApplicationState {
    private static final long INITIAL_SEQ = 0;

    private final String localNodeId;
    private final Map<String, Player> peers = new HashMap<>();
    private final Set<String> members = new HashSet<>();
    private Player local;

    public ApplicationState(String localNodeId, Position initialPosition) {
        this.localNodeId = localNodeId;
        this.local = new Player(
                localNodeId,
                initialPosition,
                INITIAL_SEQ,
                System.nanoTime());
    }

    public synchronized Player local() {
        return local;
    }

    public synchronized Optional<Player> peer(String nodeId) {
        return Optional.ofNullable(peers.get(nodeId));
    }

    public synchronized List<Player> peers() {
        return peers.keySet().stream()
                .sorted()
                .map(peers::get)
                .toList();
    }

    public synchronized int memberCount() {
        return members.size();
    }

    public synchronized boolean isLocal(String author) {
        return localNodeId.equals(author);
    }

    public synchronized void peerJoined(String nodeId) {
        if (!isLocal(nodeId)) {
            members.add(nodeId);
        }
    }

    public synchronized void peerLeft(String nodeId) {
        if (isLocal(nodeId)) {
            return;
        }

        members.remove(nodeId);
        peers.remove(nodeId);
    }

    public synchronized boolean apply(PositionEvent event) {
        long nowNanos = System.nanoTime();

        if (isLocal(event.author())) {
            if (!local.accepts(event)) {
                return false;
            }

            local = local.updated(event, nowNanos);
            return true;
        }

        Player current = peers.get(event.author());
        if (current != null && !current.accepts(event)) {
            return false;
        }

        Player updated = current == null
                ? new Player(event.author(), event.position(), event.seq(), nowNanos)
                : current.updated(event, nowNanos);
        peers.put(event.author(), updated);
        members.add(event.author());
        return true;
    }
}
