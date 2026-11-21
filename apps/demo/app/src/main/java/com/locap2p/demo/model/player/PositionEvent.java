package com.locap2p.demo.model.player;

public record PositionEvent(String author, Position position, long seq) {
}
