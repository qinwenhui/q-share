// Directory listing store.
//
// In the new routing model the current path comes from the URL
// (`/browse/*path` splat in @solidjs/router). This module owns only the
// fetched state (entries / loading / error) and the sort UI signals.
// Path-aware consumers (Browse.tsx) read the splat via `useParams()` and
// call `loadPath(path)` here.
//
// Search lives on a dedicated /search route — the sidebar's SearchPanel
// navigates there instead of fetching results inline.

import { createSignal } from 'solid-js';
import { createStore } from 'solid-js/store';
import type { DirListing, SortKey, SortOrder } from '../types';
import { listDir } from '../api';

interface DirStore {
  entries: DirListing | null;
  loading: boolean;
  error: string | null;
}

export const [sort, setSort] = createSignal<SortKey>('name');
export const [order, setOrder] = createSignal<SortOrder>('asc');

const [state, setState] = createStore<DirStore>({
  entries: null,
  loading: false,
  error: null,
});

export const dirState = state;

/** Replace the current listing without re-fetching. Used by live updates. */
export function setEntries(listing: DirListing) {
  setState({ entries: listing, loading: false, error: null });
}

export async function loadPath(path: string) {
  setState({ loading: true, error: null });
  try {
    const listing = await listDir(path, {
      sort: sort(),
      order: order(),
      limit: 1000, // fetch full directory; virtual list handles rendering
    });
    setState({ entries: listing, loading: false });
  } catch (e: any) {
    setState({ loading: false, error: e?.message ?? String(e) });
  }
}

export function cycleSort(key: SortKey) {
  if (sort() === key) {
    setOrder(order() === 'asc' ? 'desc' : 'asc');
  } else {
    setSort(key);
    setOrder(key === 'name' ? 'asc' : 'desc');
  }
}

/** True when the input is a well-formed URL path the backend will accept. */
export function isValidUrlPath(s: string): boolean {
  if (!s) return false;
  // Must start with `/`, no `..` segments, no absolute-path prefixes.
  if (!s.startsWith('/')) return false;
  if (s.includes('..')) return false;
  if (/[\x00-\x1f]/.test(s)) return false;
  return true;
}

/** Normalise a user-typed path so identical paths compare equal. */
export function normaliseUrlPath(s: string): string {
  if (!s) return '/';
  let out = s.trim();
  if (!out.startsWith('/')) out = '/' + out;
  // collapse repeated slashes
  out = out.replace(/\/{2,}/g, '/');
  // strip trailing slash unless root
  if (out.length > 1 && out.endsWith('/')) out = out.replace(/\/+$/, '');
  return out || '/';
}

/** Path utilities used by sidebar / breadcrumb. */
export function joinUrlPath(base: string, name: string): string {
  return normaliseUrlPath(`${base}/${encodeURIComponent(name)}`);
}

export function parentUrlPath(p: string): string {
  const n = normaliseUrlPath(p);
  if (n === '/' || n === '') return '/';
  const idx = n.lastIndexOf('/');
  if (idx <= 0) return '/';
  return n.slice(0, idx);
}