package com.locap2p.demo.model.player;

/**
 * A player known to the application. Instances are immutable.
 */
public record Player(
        String nodeId,
        Position position,
        long seq,
        long observedAtNanos) {

    private static final long NANOS_PER_MILLI = 1_000_000L;

    public boolean accepts(PositionEvent event) {
        return event.seq() > seq;
    }

    public Player updated(PositionEvent event, long nowNanos) {
        return new Player(nodeId, event.position(), event.seq(), nowNanos);
    }

    public long ageMillis(long nowNanos) {
        return Math.max(0, (nowNanos - observedAtNanos) / NANOS_PER_MILLI);
    }
}
