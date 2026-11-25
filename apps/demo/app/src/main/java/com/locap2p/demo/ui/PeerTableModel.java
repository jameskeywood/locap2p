package com.locap2p.demo.ui;

import java.util.List;
import java.util.Locale;

import javax.swing.table.AbstractTableModel;

import com.locap2p.demo.model.player.Player;

/**
 * Table model listing known peers.
 */
public final class PeerTableModel extends AbstractTableModel {
    private static final String[] COLUMNS = {
        "Peer", "X", "Y", "Seq", "Distance", "Age"
    };
    private static final int COLUMN_PEER = 0;
    private static final int COLUMN_X = 1;
    private static final int COLUMN_Y = 2;
    private static final int COLUMN_SEQ = 3;
    private static final int COLUMN_DISTANCE = 4;
    private static final int COLUMN_AGE = 5;
    private static final long MILLIS_PER_SECOND = 1_000L;
    private static final double MILLIS_PER_SECOND_DOUBLE = 1_000.0;

    private Player local;
    private List<Player> peers = List.of();

    public void setPlayers(Player local, List<Player> peers) {
        this.local = local;
        this.peers = List.copyOf(peers);
        fireTableDataChanged();
    }

    @Override
    public int getRowCount() {
        return peers.size();
    }

    @Override
    public int getColumnCount() {
        return COLUMNS.length;
    }

    @Override
    public String getColumnName(int column) {
        return COLUMNS[column];
    }

    @Override
    public Object getValueAt(int rowIndex, int columnIndex) {
        Player peer = peers.get(rowIndex);

        return switch (columnIndex) {
            case COLUMN_PEER -> peer.nodeId();
            case COLUMN_X -> Integer.toString(peer.position().x());
            case COLUMN_Y -> Integer.toString(peer.position().y());
            case COLUMN_SEQ -> Long.toString(peer.seq());
            case COLUMN_DISTANCE -> formatDistance(peer);
            case COLUMN_AGE -> formatAge(peer.ageMillis(System.nanoTime()));
            default -> "";
        };
    }

    private String formatDistance(Player peer) {
        if (local == null) {
            return "-";
        }

        double distance = local.position().distanceTo(peer.position());
        return String.format(Locale.ROOT, "%.1f", distance);
    }

    private static String formatAge(long ageMillis) {
        if (ageMillis < MILLIS_PER_SECOND) {
            return ageMillis + "ms";
        }

        return String.format(
                Locale.ROOT,
                "%.1fs",
                ageMillis / MILLIS_PER_SECOND_DOUBLE);
    }
}
