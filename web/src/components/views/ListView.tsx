// List view — dense row-based rendering for directory listings.
//
// Identical contract to GridView: takes entries + selection + activation
// callbacks. The parent (Browse) owns the selected-index state so keyboard
// navigation, view-switching, and live updates can all share one source
// of truth.
//
// We use absolute positioning inside a fixed-height container so we only
// render rows that the user can actually see (small/medium directories
// this is moot; for very large directories the perf win matters). The
// scroll container is exposed via `onScrollRef` so the parent can scroll
// the selected row into view via querySelector.

import { Component, For, createSignal, onMount, onCleanup } from 'solid-js';
import type { DirEntry } from '../../types';
import { FileRow } from '../FileRow';

const ROW_HEIGHT = 48;

interface Props {
  entries: DirEntry[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onActivate: (entry: DirEntry) => void;
  /** Notified once the scroll container mounts so the parent can run
   *  scrollSelectedIntoView against it. */
  onScrollMount?: (el: HTMLDivElement) => void;
  /** Long-press (mobile) / right-click (desktop) on a row. */
  onContext?: (entry: DirEntry, x: number, y: number) => void;
}

export const ListView: Component<Props> = (props) => {
  const [containerEl, setContainerEl] = createSignal<HTMLDivElement | undefined>();

  onMount(() => {
    const el = containerEl();
    if (el) props.onScrollMount?.(el);
    onCleanup(() => props.onScrollMount?.(undefined as unknown as HTMLDivElement));
  });

  return (
    <div
      class="list-view"
      ref={(el) => setContainerEl(el)}
      role="grid"
      aria-label="files"
    >
      <div style={{ position: 'relative', height: props.entries.length * ROW_HEIGHT + 'px' }}>
        <For each={props.entries}>
          {(entry, i) => (
            <div
              style={{
                position: 'absolute',
                top: i() * ROW_HEIGHT + 'px',
                left: 0,
                right: 0,
                height: ROW_HEIGHT + 'px',
              }}
            >
              <FileRow
                entry={entry}
                index={i()}
                selected={props.selectedIndex === i()}
                onSelect={props.onSelect}
                onActivate={props.onActivate}
                onContext={props.onContext}
              />
            </div>
          )}
        </For>
      </div>
    </div>
  );
};

export const LIST_ROW_HEIGHT = ROW_HEIGHT;
export default ListView;
