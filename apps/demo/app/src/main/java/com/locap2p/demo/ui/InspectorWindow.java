package com.locap2p.demo.ui;

import java.awt.BorderLayout;
import java.awt.Dimension;
import java.awt.GraphicsEnvironment;
import java.util.List;

import javax.swing.BorderFactory;
import javax.swing.JFrame;
import javax.swing.JLabel;
import javax.swing.JPanel;
import javax.swing.JScrollPane;
import javax.swing.JTable;
import javax.swing.SwingUtilities;
import javax.swing.Timer;
import javax.swing.WindowConstants;

import com.locap2p.demo.ApplicationState;
import com.locap2p.demo.model.player.Player;

/**
 * Read-only window showing the current application state.
 */
public final class InspectorWindow {
    private static final int REFRESH_MILLIS = 250;
    private static final int WINDOW_WIDTH = 760;
    private static final int WINDOW_HEIGHT = 520;
    private static final int ROW_HEIGHT = 22;

    private final ApplicationState state;
    private final String session;
    private final JFrame frame;
    private final JLabel summary;
    private final PeerTableModel tableModel;
    private final MapPanel mapPanel;
    private Timer timer;

    private InspectorWindow(ApplicationState state, String session) {
        this.state = state;
        this.session = session;
        this.tableModel = new PeerTableModel();
        this.mapPanel = new MapPanel();
        this.summary = new JLabel();
        this.frame = new JFrame("locap2p inspector");

        frame.setDefaultCloseOperation(WindowConstants.HIDE_ON_CLOSE);
        frame.setLayout(new BorderLayout(8, 8));
        frame.add(buildHeader(), BorderLayout.NORTH);
        frame.add(buildBody(), BorderLayout.CENTER);
        frame.setPreferredSize(new Dimension(WINDOW_WIDTH, WINDOW_HEIGHT));
        frame.pack();
        frame.setLocationByPlatform(true);
    }

    public static void start(ApplicationState state, String session) {
        if (GraphicsEnvironment.isHeadless()) {
            System.err.println(
                    "display unavailable; continuing without the inspector window");
            return;
        }

        SwingUtilities.invokeLater(() -> {
            InspectorWindow window = new InspectorWindow(state, session);
            window.refresh();
            window.frame.setVisible(true);
            window.startRefreshTimer();
        });
    }

    private JPanel buildHeader() {
        JPanel panel = new JPanel(new BorderLayout());
        panel.setBorder(BorderFactory.createEmptyBorder(8, 10, 0, 10));
        panel.add(summary, BorderLayout.CENTER);
        return panel;
    }

    private JPanel buildBody() {
        JTable table = new JTable(tableModel);
        table.setFillsViewportHeight(true);
        table.setRowHeight(ROW_HEIGHT);

        JScrollPane scrollPane = new JScrollPane(table);
        scrollPane.setBorder(BorderFactory.createTitledBorder("Known peers"));

        JPanel panel = new JPanel(new BorderLayout(0, 8));
        panel.setBorder(BorderFactory.createEmptyBorder(8, 10, 10, 10));
        panel.add(scrollPane, BorderLayout.CENTER);
        panel.add(mapPanel, BorderLayout.SOUTH);
        return panel;
    }

    private void startRefreshTimer() {
        timer = new Timer(REFRESH_MILLIS, event -> refresh());
        timer.start();
    }

    private void refresh() {
        Player local = state.local();
        List<Player> peers = state.peers();

        summary.setText(String.format(
                "node %s   session %s   local (%d, %d)   players %d   members %d",
                local.nodeId(),
                session,
                local.position().x(),
                local.position().y(),
                peers.size(),
                state.memberCount()));

        tableModel.setPlayers(local, peers);
        mapPanel.setPlayers(local, peers);
    }
}
