// WebSocket client + live directory store.
//
// Connects to `/ws` on app start. The route component (Browse) tells us
// which path to watch via `watchPath(p)` whenever the route changes.
//
// Why WebSocket instead of SSE: bidirectional means we can explicitly tell
// the server "I'm done with /photos" instead of relying on connection
// drops. Cleaner subscription lifecycle, no leaked watchers.
//
// Stats are pushed by the server every ~2 s; we mirror them into
// `liveStats` so the status bar can subscribe via signal rather than
// polling /api/stats.

import { createSignal, onCleanup } from 'solid-js';
import { listDir } from '../api';
import { dirState, sort, order, setEntries } from './dir';

type Status = 'connecting' | 'live' | 'offline';

const [status, setStatus] = createSignal<Status>('connecting');
export const wsStatus = status;

interface LiveStats {
  active: number;
  bytes_served: number;
  errors: number;
}
const [stats, setStats] = createSignal<LiveStats>({
  active: 0,
  bytes_served: 0,
  errors: 0,
});
export const liveStats = stats;

interface FsEvent {
  kind: 'created' | 'modified' | 'removed' | 'renamed';
  path: string;
  is_dir?: boolean;
}

interface ServerFrame {
  type: 'welcome' | 'watching' | 'unwatched' | 'fs-event' | 'stats' | 'log' | 'pong' | 'error';
  [k: string]: unknown;
}

let socket: WebSocket | null = null;
let retryTimer: ReturnType<typeof setTimeout> | null = null;
let activelyWatching: string | null = null;
const RETRY_DELAY_MS = 1500;

function url(): string {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  return `${proto}://${location.host}/ws`;
}

function send(msg: object): boolean {
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(msg));
    return true;
  }
  return false;
}

function connect() {
  if (socket && socket.readyState <= WebSocket.OPEN) return;
  setStatus('connecting');
  try {
    socket = new WebSocket(url());
  } catch {
    scheduleReconnect();
    return;
  }

  socket.addEventListener('open', () => {
    setStatus('live');
    if (activelyWatching) send({ op: 'watch', path: activelyWatching });
  });

  socket.addEventListener('message', (e) => {
    let frame: ServerFrame;
    try {
      frame = JSON.parse(e.data);
    } catch {
      return;
    }
    switch (frame.type) {
      case 'welcome':
      case 'watching':
      case 'unwatched':
        break;
      case 'fs-event':
        onFsEvent(frame);
        break;
      case 'stats': {
        const s = frame as unknown as LiveStats;
        if (typeof s.active === 'number') setStats(s);
        break;
      }
      case 'log':
      case 'pong':
        break;
      case 'error':
        console.warn('server error:', frame);
        break;
    }
  });

  socket.addEventListener('close', () => {
    setStatus('offline');
    activelyWatching = null;
    scheduleReconnect();
  });
}

function scheduleReconnect() {
  if (retryTimer) return;
  retryTimer = setTimeout(() => {
    retryTimer = null;
    connect();
  }, RETRY_DELAY_MS);
}

function onFsEvent(frame: ServerFrame) {
  const events = (frame.events as FsEvent[] | undefined) ?? [];
  const watched = (frame.path as string | undefined) ?? '';
  const cur = activelyWatching ?? '';
  const matches = events.some((e) => {
    const p = e.path.startsWith('/') ? e.path : `/${e.path}`;
    if (cur === '/' || cur === '') return true;
    return p === cur || p.startsWith(cur + '/') || watched === cur;
  });
  if (!matches) return;
  void refreshCurrent(cur);
}

async function refreshCurrent(path: string) {
  try {
    const listing = await listDir(path, {
      sort: sort(),
      order: order(),
      limit: 1000,
    });
    setEntries(listing);
  } catch (e) {
    console.warn('live refresh failed', e);
  }
}

/** Watch a path — replaces any previous watch. */
export function watchPath(path: string) {
  if (activelyWatching === path) {
    send({ op: 'watch', path });
    return;
  }
  if (activelyWatching) send({ op: 'unwatch', path: activelyWatching });
  activelyWatching = path;
  send({ op: 'watch', path });
}

export function unwatch() {
  if (activelyWatching) {
    send({ op: 'unwatch', path: activelyWatching });
    activelyWatching = null;
  }
}

/** Called by the Layout once. */
export function startLive() {
  connect();
}

export function stopLive() {
  if (socket) {
    try {
      socket.close();
    } catch {}
    socket = null;
  }
  if (retryTimer) {
    clearTimeout(retryTimer);
    retryTimer = null;
  }
}