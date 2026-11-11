package locap2p.demo;

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
}
