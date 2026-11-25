package com.locap2p.demo;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class MainTest {
    @Test
    void parsesHeadlessAndRandomIndependently() {
        Main.Arguments headless = parse("--headless");
        assertTrue(headless.headless());
        assertFalse(headless.random());

        Main.Arguments random = parse("--random");
        assertFalse(random.headless());
        assertTrue(random.random());

        Main.Arguments both = parse("--headless", "--random");
        assertTrue(both.headless());
        assertTrue(both.random());
    }

    @Test
    void rejectsUnknownOrIncompleteArguments() {
        assertNull(Main.parseArguments(new String[] {"--random"}));
        assertNull(Main.parseArguments(new String[] {
            "--host-bin", "host", "--config", "config", "--unknown"
        }));
    }

    private static Main.Arguments parse(String... options) {
        String[] args = new String[4 + options.length];
        args[0] = "--host-bin";
        args[1] = "host";
        args[2] = "--config";
        args[3] = "config";
        System.arraycopy(options, 0, args, 4, options.length);
        return Main.parseArguments(args);
    }
}
