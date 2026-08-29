// Favorites store — pins paths to a sidebar list.
//
// Persists to localStorage so reloads survive. Schema is intentionally
// versioned (`v1`) so a future incompatible change can fall back to []
// instead of crashing on malformed legacy data.
//
// Mutations go through the public API (add / remove / reorder / rename).
// Reads are reactive via the `favorites` signal — anywhere in the tree
// can read `favorites()` and re-render on change.

import { createSignal } from 'solid-js';

export interface Favorite {
  id: string;
  path: string;
  label: string;
  pinnedAt: number;
}

const STORAGE_KEY = 'qshare.favorites.v1';

function load(): Favorite[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    // Light validation; drop bad rows rather than throw.
    const valid = parsed.filter(
      (f): f is Favorite =>
        f &&
        typeof f.id === 'string' &&
        typeof f.path === 'string' &&
        typeof f.label === 'string' &&
        typeof f.pinnedAt === 'number',
    );
    // v1 labels were the percent-encoded URL path segment (e.g. a Chinese
    // directory stored as "%E4%B8%AD..." — that bug shipped). Migrate any
    // stored label that still looks encoded to its decoded display form.
    return valid.map((f) => ({ ...f, label: decodeLabel(f.label) }));
  } catch {
    return [];
  }
}

function save(items: Favorite[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
  } catch {
    // Quota exceeded / private mode — silently ignore. The signal still
    // updates in memory so the UI works for this session.
  }
}

const [items, setItems] = createSignal<Favorite[]>(load());

export const favorites = items;

/** Add a path to the favorites list. No-op if already present. */
export function addFavorite(path: string, label?: string): Favorite | null {
  const norm = path.trim();
  if (!norm || !norm.startsWith('/')) return null;
  if (items().some((f) => f.path === norm)) return null;
  const fav: Favorite = {
    id: uuid(),
    path: norm,
    label: label ?? deriveLabel(norm),
    pinnedAt: Date.now(),
  };
  const next = [...items(), fav];
  setItems(next);
  save(next);
  return fav;
}

export function removeFavorite(id: string) {
  const next = items().filter((f) => f.id !== id);
  if (next.length !== items().length) {
    setItems(next);
    save(next);
  }
}

export function renameFavorite(id: string, label: string) {
  const trimmed = label.trim();
  if (!trimmed) return;
  const next = items().map((f) => (f.id === id ? { ...f, label: trimmed } : f));
  setItems(next);
  save(next);
}

/** Move the favorite at `from` to position `to`. */
export function reorderFavorite(from: number, to: number) {
  if (from === to) return;
  const list = [...items()];
  if (from < 0 || from >= list.length) return;
  if (to < 0 || to >= list.length) return;
  const [moved] = list.splice(from, 1);
  list.splice(to, 0, moved);
  setItems(list);
  save(list);
}

export function isFavorite(path: string): boolean {
  return items().some((f) => f.path === path);
}

const HAS_PCT_ESCAPE = /%[0-9a-fA-F]{2}/;

/** Decode a percent-encoded label for display. Only decodes when the
 *  string actually contains an escape sequence, so a literal `%` in a
 *  filename (already-decoded) survives untouched. */
function decodeLabel(s: string): string {
  if (!HAS_PCT_ESCAPE.test(s)) return s;
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}

/** Use the last segment as the default label. The path is the URL path
 *  (percent-encoded), so decode it for display — otherwise a Chinese
 *  directory name shows up as "%E4%B8%AD..." in the sidebar. */
function deriveLabel(path: string): string {
  const trimmed = path.replace(/\/+$/, '');
  const last = trimmed.split('/').pop();
  if (!last) return '/';
  return decodeLabel(last);
}

/**
 * Generate a UUID. Prefers the native `crypto.randomUUID()` (fast, RFC 4122
 * v4) and falls back to a Math.random-based v4 if the native API is
 * missing — which it is on `http://192.168.x.x` (non-loopback LAN IPs are
 * considered insecure contexts and don't get `randomUUID`). q-share is a
 * LAN tool, so phones hitting it over `http://192.168.x.x:8888` would
 * throw on every click without this fallback, which silently killed the
 * PinButton (the throw aborted `addFavorite` before it ever touched the
 * signal — favourites list never updated).
 */
function uuid(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}
