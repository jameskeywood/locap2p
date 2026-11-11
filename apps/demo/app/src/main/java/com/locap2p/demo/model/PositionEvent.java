package com.locap2p.demo.model;

public record PositionEvent(String author, Position position, long seq) {
}
