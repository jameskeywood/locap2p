package com.locap2p.demo.model.ipc;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.JsonNode;

/**
 * An event received from the framework.
 */
public record FrameworkEvent(
        String id,
        String author,
        long seq,
        @JsonProperty("timestamp_nanos") long timestampNanos,
        JsonNode payload) {
}
