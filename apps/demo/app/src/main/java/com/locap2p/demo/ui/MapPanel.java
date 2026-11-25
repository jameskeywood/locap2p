package com.locap2p.demo.ui;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.geom.Ellipse2D;
import java.awt.geom.Line2D;
import java.util.ArrayList;
import java.util.List;

import javax.swing.JPanel;

import com.locap2p.demo.model.player.Player;
import com.locap2p.demo.model.player.Position;

/**
 * Draws the local player and every known peer.
 */
public final class MapPanel extends JPanel {
    private static final Color BACKGROUND = new Color(0xF7F7F2);
    private static final Color LOCAL_COLOR = new Color(0x2E7D32);
    private static final Color PEER_COLOR = new Color(0x1565C0);
    private static final Color LINK_COLOR = new Color(0xB0BEC5);
    private static final int PADDING = 24;
    private static final double NODE_RADIUS = 7.0;
    private static final int PREFERRED_HEIGHT = 220;
    private static final int LABEL_LENGTH = 8;

    private Player local;
    private List<Player> peers = List.of();

    public MapPanel() {
        setBackground(BACKGROUND);
        setPreferredSize(new Dimension(0, PREFERRED_HEIGHT));
    }

    public void setPlayers(Player local, List<Player> peers) {
        this.local = local;
        this.peers = List.copyOf(peers);
        repaint();
    }

    @Override
    protected void paintComponent(Graphics graphics) {
        super.paintComponent(graphics);

        if (local == null) {
            return;
        }

        Graphics2D g = (Graphics2D) graphics.create();
        try {
            g.setRenderingHint(
                    RenderingHints.KEY_ANTIALIASING,
                    RenderingHints.VALUE_ANTIALIAS_ON);
            drawPlayers(g);
        } finally {
            g.dispose();
        }
    }

    private void drawPlayers(Graphics2D g) {
        List<Position> positions = new ArrayList<>();
        positions.add(local.position());
        for (Player peer : peers) {
            positions.add(peer.position());
        }

        Projection projection = Projection.fit(
                Bounds.of(positions),
                getWidth(),
                getHeight());

        Position localPosition = local.position();
        double localX = projection.x(localPosition.x());
        double localY = projection.y(localPosition.y());

        g.setStroke(new BasicStroke(1.0f));
        for (Player peer : peers) {
            double peerX = projection.x(peer.position().x());
            double peerY = projection.y(peer.position().y());
            g.setColor(LINK_COLOR);
            g.draw(new Line2D.Double(localX, localY, peerX, peerY));
            drawNode(g, peerX, peerY, PEER_COLOR, label(peer.nodeId()));
        }

        drawNode(g, localX, localY, LOCAL_COLOR, "you");
    }

    private static void drawNode(Graphics2D g, double x, double y, Color color, String label) {
        g.setColor(color);
        g.fill(new Ellipse2D.Double(
                x - NODE_RADIUS,
                y - NODE_RADIUS,
                NODE_RADIUS * 2.0,
                NODE_RADIUS * 2.0));
        g.drawString(label, (float) (x + NODE_RADIUS + 3.0), (float) (y + 4.0));
    }

    private static String label(String nodeId) {
        if (nodeId.length() <= LABEL_LENGTH) {
            return nodeId;
        }

        return nodeId.substring(0, LABEL_LENGTH);
    }

    private record Bounds(int minX, int maxX, int minY, int maxY) {
        static Bounds of(List<Position> positions) {
            int minX = Integer.MAX_VALUE;
            int maxX = Integer.MIN_VALUE;
            int minY = Integer.MAX_VALUE;
            int maxY = Integer.MIN_VALUE;

            for (Position position : positions) {
                minX = Math.min(minX, position.x());
                maxX = Math.max(maxX, position.x());
                minY = Math.min(minY, position.y());
                maxY = Math.max(maxY, position.y());
            }

            return new Bounds(minX, maxX, minY, maxY);
        }
    }

    private record Projection(
            double minX,
            double maxY,
            double scale,
            double offsetX,
            double offsetY) {

        static Projection fit(Bounds bounds, int width, int height) {
            double usableWidth = Math.max(1.0, width - 2.0 * PADDING);
            double usableHeight = Math.max(1.0, height - 2.0 * PADDING);
            double worldWidth = bounds.maxX() - bounds.minX();
            double worldHeight = bounds.maxY() - bounds.minY();

            double scaleX = worldWidth > 0 ? usableWidth / worldWidth : Double.MAX_VALUE;
            double scaleY = worldHeight > 0 ? usableHeight / worldHeight : Double.MAX_VALUE;
            double scale = Math.min(scaleX, scaleY);
            if (!Double.isFinite(scale)) {
                scale = 1.0;
            }

            double drawnWidth = worldWidth * scale;
            double drawnHeight = worldHeight * scale;
            double offsetX = (width - drawnWidth) / 2.0;
            double offsetY = (height - drawnHeight) / 2.0;

            return new Projection(
                    bounds.minX(),
                    bounds.maxY(),
                    scale,
                    offsetX,
                    offsetY);
        }

        double x(int value) {
            return offsetX + (value - minX) * scale;
        }

        double y(int value) {
            return offsetY + (maxY - value) * scale;
        }
    }
}
