// View store — list vs grid, grid tile size.
//
// Persisted to localStorage so the user's chosen view survives reloads.
// The schema is intentionally tiny: a single object keyed `qshare.view.v1`
// holding both `mode` and `size`. We coalesce into one key because the two
// settings always change together in practice and a single read is cheaper
// than two signals racing each other.

import { createSignal } from 'solid-js';

export type ViewMode = 'list' | 'grid';
export type GridSize = 'sm' | 'md' | 'lg';

const STORAGE_KEY = 'qshare.view.v1';

interface Persisted {
  mode: ViewMode;
  size: GridSize;
}

const DEFAULT: Persisted = { mode: 'grid', size: 'md' };

function load(): Persisted {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT;
    const p = JSON.parse(raw);
    if (p && (p.mode === 'list' || p.mode === 'grid') &&
        (p.size === 'sm' || p.size === 'md' || p.size === 'lg')) {
      return { mode: p.mode, size: p.size };
    }
  } catch {}
  return DEFAULT;
}

function save(p: Persisted) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(p));
  } catch {}
}

const [persisted, setPersisted] = createSignal<Persisted>(load());

export const viewMode = () => persisted().mode;
export const gridSize = () => persisted().size;

export function setViewMode(m: ViewMode) {
  const next = { ...persisted(), mode: m };
  setPersisted(next);
  save(next);
}

export function setGridSize(s: GridSize) {
  const next = { ...persisted(), size: s };
  setPersisted(next);
  save(next);
}

export function toggleViewMode() {
  setViewMode(viewMode() === 'grid' ? 'list' : 'grid');
}

/** Tile minimum width per grid size — drives the auto-fill grid layout. */
export const GRID_TILE_PX: Record<GridSize, number> = {
  sm: 110,
  md: 160,
  lg: 220,
};
