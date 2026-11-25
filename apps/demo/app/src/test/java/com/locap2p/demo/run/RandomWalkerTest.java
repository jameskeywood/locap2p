package com.locap2p.demo.run;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.time.Duration;
import java.util.Random;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

import org.junit.jupiter.api.Test;

import com.locap2p.demo.model.player.Position;

class RandomWalkerTest {
    private static final Duration FAST_STEP = Duration.ofMillis(1);

    @Test
    void movesExactlyOneCellPerStep() {
        AtomicInteger ignored = new AtomicInteger();
        RandomWalker walker = new RandomWalker(
                position -> ignored.incrementAndGet(),
                new Random(42),
                FAST_STEP);
        Position current = new Position(0, 0);

        for (int step = 0; step < 100; step++) {
            Position next = walker.next(current);
            int dx = Math.abs(next.x() - current.x());
            int dy = Math.abs(next.y() - current.y());

            assertEquals(1, dx + dy);
            assertTrue(dx <= 1);
            assertTrue(dy <= 1);
            current = next;
        }

        assertNotEquals(new Position(0, 0), current);
    }

    @Test
    void runPublishesUntilInterrupted() throws InterruptedException {
        CountDownLatch published = new CountDownLatch(3);
        RandomWalker walker = new RandomWalker(
                position -> published.countDown(),
                new Random(7),
                FAST_STEP);

        Thread thread = new Thread(() -> walker.run(new Position(0, 0)));
        thread.start();

        assertTrue(published.await(2, TimeUnit.SECONDS));

        thread.interrupt();
        thread.join(1_000);

        assertFalse(thread.isAlive());
    }

    @Test
    void runStopsWhenPublishingFails() {
        RandomWalker walker = new RandomWalker(
                position -> {
                    throw new IOException("host is gone");
                },
                new Random(7),
                FAST_STEP);

        assertDoesNotThrow(() -> walker.run(new Position(0, 0)));
    }
}
