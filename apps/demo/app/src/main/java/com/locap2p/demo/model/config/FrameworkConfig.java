package com.locap2p.demo.model.config;

import java.io.IOException;
import java.nio.file.Path;
import java.util.List;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;

/**
 * Application-defined configuration passed to the framework host over IPC.
 *
 * <p>The JSON representation is the shared configuration contract between the
 * application and the Rust framework.
 */
public record FrameworkConfig(
        int schemaVersion,
        Application application,
        Identity identity,
        Network network,
        Discovery discovery,
        Connectivity connectivity,
        Gossip gossip) {

    private static final ObjectMapper MAPPER = new ObjectMapper()
            .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
            .disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

    public static FrameworkConfig load(Path path) throws IOException {
        return MAPPER.readValue(path.toFile(), FrameworkConfig.class);
    }

    public String toJson() throws IOException {
        return MAPPER.writeValueAsString(this);
    }

    public record Application(String id, String session) {
    }

    public record Identity(String path) {
    }

    public record Network(List<String> listenAddrs, long requestTimeoutMs) {
    }

    public record Discovery(List<String> bootstrap) {
    }

    public record Connectivity(
            List<String> relay,
            boolean reserveRelay,
            boolean preferDirect,
            boolean enableDcutr) {
    }

    public record Gossip(
            long syncIntervalMs,
            int fanout,
            int maxEventsPerResponse,
            long membershipTtlMs,
            long expiryGraceMs) {
    }
}
