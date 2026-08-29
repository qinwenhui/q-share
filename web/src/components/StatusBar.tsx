import { Component } from 'solid-js';
import { ConnectionStatus } from './ConnectionStatus';

/**
 * Fixed bottom bar. The connection/error/bytes stats used to live here,
 * but those are server-side counters that belong in the server's own
 * dashboard — the web UI is a file browser, not a monitoring console — so
 * the bar just shows the live-update indicator (WebSocket state) and the
 * brand link now.
 */
export const StatusBar: Component = () => {
  return (
    <footer class="status-bar">
      <div class="status-left">
        <ConnectionStatus />
      </div>
      <div class="status-right">
        <a
          class="status-link"
          href="https://github.com/qinwenhui/q-share"
          target="_blank"
          rel="noopener"
        >
          q-share
        </a>
      </div>
    </footer>
  );
};

export default StatusBar;
