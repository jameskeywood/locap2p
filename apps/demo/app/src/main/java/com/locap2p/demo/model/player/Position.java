package com.locap2p.demo.model.player;

/**
 * A player position on the game grid.
 */
public record Position(int x, int y) {
    public Position movedBy(Direction direction) {
        return switch (direction) {
            case UP -> new Position(x, y + 1);
            case DOWN -> new Position(x, y - 1);
            case LEFT -> new Position(x - 1, y);
            case RIGHT -> new Position(x + 1, y);
        };
    }

    public double distanceTo(Position other) {
        return Math.hypot(x - other.x(), y - other.y());
    }
}
