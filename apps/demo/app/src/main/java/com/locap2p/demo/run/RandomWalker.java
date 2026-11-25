package com.locap2p.demo.run;

import java.io.IOException;
import java.time.Duration;
import java.util.Objects;
import java.util.Random;

import com.locap2p.demo.model.player.Direction;
import com.locap2p.demo.model.player.Position;

/**
 * Publishes random movement for a single player.
 */
public final class RandomWalker {
    private static final Duration STEP_INTERVAL = Duration.ofMillis(1_000);
    private static final Direction[] DIRECTIONS = Direction.values();

    private final PositionPublisher publisher;
    private final Random random;
    private final Duration stepInterval;

    public RandomWalker(PositionPublisher publisher) {
        this(publisher, new Random(), STEP_INTERVAL);
    }

    RandomWalker(PositionPublisher publisher, Random random, Duration stepInterval) {
        this.publisher = Objects.requireNonNull(publisher, "publisher");
        this.random = Objects.requireNonNull(random, "random");
        this.stepInterval = Objects.requireNonNull(stepInterval, "stepInterval");
    }

    public void run(Position initialPosition) {
        Objects.requireNonNull(initialPosition, "initialPosition");
        Position current = initialPosition;

        while (!Thread.currentThread().isInterrupted()) {
            current = next(current);

            try {
                publisher.publish(current);
            } catch (IOException error) {
                System.err.printf("random walk stopped: %s%n", error.getMessage());
                return;
            }

            System.out.printf("random move: (%d, %d)%n", current.x(), current.y());

            try {
                Thread.sleep(stepInterval);
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                return;
            }
        }
    }

    Position next(Position current) {
        Direction direction = DIRECTIONS[random.nextInt(DIRECTIONS.length)];
        return current.movedBy(direction);
    }
}
