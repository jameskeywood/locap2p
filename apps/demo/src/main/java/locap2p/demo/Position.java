package locap2p.demo;

public record Position(int x, int y) {
    public Position movedBy(Direction direction) {
        return switch (direction) {
            case UP -> new Position(x, y + 1);
            case DOWN -> new Position(x, y - 1);
            case LEFT -> new Position(x - 1, y);
            case RIGHT -> new Position(x + 1, y);
        };
    }
}
