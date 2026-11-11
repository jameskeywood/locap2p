package locap2p.demo;

import java.util.HashMap;
import java.util.Map;

public final class WorldState {
    private final String localNodeId;
    private final Map<String, PlayerView> peers = new HashMap<>();
    private Position localPosition = new Position(0, 0);

    public WorldState(String localNodeId) {
        this.localNodeId = localNodeId;
    }

    public synchronized Position localPosition() {
        return localPosition;
    }

    public synchronized Position moveLocal(Direction direction) {
        localPosition = localPosition.movedBy(direction);
        return localPosition;
    }

    public synchronized boolean applyRemote(String author, long seq, Position position) {
        if (author.equals(localNodeId)) {
            return false;
        }

        PlayerView current = peers.get(author);
        if (current != null && seq <= current.seq()) {
            return false;
        }

        peers.put(author, new PlayerView(position, seq));
        return true;
    }

    public synchronized Map<String, PlayerView> peers() {
        return Map.copyOf(peers);
    }
}
