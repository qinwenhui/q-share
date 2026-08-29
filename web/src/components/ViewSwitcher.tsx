// View switcher — list/grid toggle + grid size selector.
//
// Each mode button is icon-only so the toolbar stays compact; the
// `title` attribute provides the accessible name. The active mode has
// a stronger accent border, filled background, and brighter text;
// inactive modes are muted.
//
// Grid size is only meaningful in grid mode, so we hide the size
// selector when list is active.

import { Component, Show } from 'solid-js';
import {
  viewMode,
  setViewMode,
  gridSize,
  setGridSize,
  type GridSize,
} from '../stores/view';

const SIZE_CYCLE: GridSize[] = ['sm', 'md', 'lg'];
const SIZE_LABEL: Record<GridSize, string> = { sm: 'S', md: 'M', lg: 'L' };

export const ViewSwitcher: Component = () => {
  return (
    <div class="view-switcher" role="group" aria-label="view options">
      <button
        type="button"
        class={`vs-btn vs-btn-list${viewMode() === 'list' ? ' on' : ''}`}
        aria-pressed={viewMode() === 'list'}
        title="list view"
        aria-label="list view"
        onClick={() => setViewMode('list')}
      >
        <svg viewBox="0 0 20 20" width="16" height="16" fill="none"
             stroke="currentColor" stroke-width="2"
             stroke-linecap="round">
          <line x1="3" y1="6"  x2="17" y2="6" />
          <line x1="3" y1="10" x2="17" y2="10" />
          <line x1="3" y1="14" x2="17" y2="14" />
        </svg>
      </button>
      <button
        type="button"
        class={`vs-btn vs-btn-grid${viewMode() === 'grid' ? ' on' : ''}`}
        aria-pressed={viewMode() === 'grid'}
        title="grid view"
        aria-label="grid view"
        onClick={() => setViewMode('grid')}
      >
        <svg viewBox="0 0 20 20" width="16" height="16" fill="none"
             stroke="currentColor" stroke-width="2"
             stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="3"  width="6" height="6" rx="1" />
          <rect x="11" y="3"  width="6" height="6" rx="1" />
          <rect x="3" y="11" width="6" height="6" rx="1" />
          <rect x="11" y="11" width="6" height="6" rx="1" />
        </svg>
      </button>

      <Show when={viewMode() === 'grid'}>
        <div class="vs-size" role="group" aria-label="grid size">
          {SIZE_CYCLE.map((s) => (
            <button
              type="button"
              class={`vs-size-btn${gridSize() === s ? ' on' : ''}`}
              aria-pressed={gridSize() === s}
              title={`${s.toUpperCase()} tiles`}
              onClick={() => setGridSize(s)}
            >
              {SIZE_LABEL[s]}
            </button>
          ))}
        </div>
      </Show>
    </div>
  );
};

export default ViewSwitcher;
