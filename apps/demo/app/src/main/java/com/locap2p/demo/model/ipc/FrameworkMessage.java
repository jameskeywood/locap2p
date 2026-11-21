package com.locap2p.demo.model.ipc;

public sealed interface FrameworkMessage {
    record Ready(String nodeId) implements FrameworkMessage {
    }

    record EventMessage(FrameworkEvent event) implements FrameworkMessage {
    }

    record PeerJoined(String nodeId) implements FrameworkMessage {
    }

    record PeerLeft(String nodeId) implements FrameworkMessage {
    }

    record Error(String message) implements FrameworkMessage {
    }
}
