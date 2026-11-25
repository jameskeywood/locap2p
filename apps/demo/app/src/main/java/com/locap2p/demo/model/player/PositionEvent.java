package com.locap2p.demo.model.player;

/**
 * A player position update.
 */
public record PositionEvent(String author, Position position, long seq) {
}
