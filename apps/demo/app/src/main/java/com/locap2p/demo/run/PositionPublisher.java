package com.locap2p.demo.run;

import java.io.IOException;

import com.locap2p.demo.model.player.Position;

/**
 * Publishes a player position.
 */
@FunctionalInterface
public interface PositionPublisher {
    void publish(Position position) throws IOException;
}
