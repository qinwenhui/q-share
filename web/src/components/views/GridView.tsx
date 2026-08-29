// Grid view — tile-based rendering for directory listings.
//
// Each tile shows a square (or near-square) preview block on top with the
// file/folder name underneath. Tiles get a type-keyed colour from
// `.tile-{kind}` (set in global.css) so folders, images, videos, etc.
// visually group without needing the user to read the icon.
//
// Auto-fill CSS grid (`repeat(auto-fill, minmax(N, 1fr))`) handles every
// viewport width without a JS resize listener. The minimum tile width is
// driven by the `gridSize` setting (sm/md/lg).
//
// Click → activate (handled by parent via onActivate, same contract as
// ListView). Selected state shares styling with the list view's selected
// row so the keyboard navigation feel is consistent.

import { Component, For, Show, createMemo } from 'solid-js';
import type { DirEntry } from '../../types';
import { tileKindOf } from '../../types';
import { FileIcon } from '../FileIcon';
import { thumbUrl } from '../../api';
import { GRID_TILE_PX, gridSize } from '../../stores/view';
import { contextTrigger } from '../ContextMenu';

interface Props {
  entries: DirEntry[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onActivate: (entry: DirEntry) => void;
  onContext?: (entry: DirEntry, x: number, y: number) => void;
}

export const GridView: Component<Props> = (props) => {
  const minPx = createMemo(() => GRID_TILE_PX[gridSize()]);

  return (
    <div
      class="grid-view"
      style={{ 'grid-template-columns': `repeat(auto-fill, minmax(${minPx()}px, 1fr))` }}
      role="grid"
      aria-label="files"
    >
      <For each={props.entries}>
        {(entry, i) => {
          const kind = tileKindOf(entry);
          const isImage = kind === 'image';
          const triggerProps = () =>
            props.onContext
              ? contextTrigger((x, y) => props.onContext!(entry, x, y))
              : {};
          return (
            <div
              class={`tile tile-${kind}${props.selectedIndex === i() ? ' selected' : ''}`}
              role="gridcell"
              aria-selected={props.selectedIndex === i()}
              tabindex={props.selectedIndex === i() ? 0 : -1}
              onClick={() => {
                props.onSelect(i());
                props.onActivate(entry);
              }}
              {...triggerProps()}
            >
              <div class="tile-thumb">
                <Show
                  when={isImage && entry.previewable}
                  fallback={<FileIcon isDir={entry.is_dir} mime={entry.mime} name={entry.name} size="lg" />}
                >
                  <img
                    class="tile-img"
                    src={thumbUrl(entry.path, 320)}
                    loading="lazy"
                    alt=""
                    onError={(e) => {
                      // Hide the broken image so the FileIcon underneath
                      // shows through.
                      (e.currentTarget as HTMLImageElement).style.display = 'none';
                    }}
                  />
                </Show>
                <Show when={kind === 'video'}>
                  <span class="tile-badge play" aria-hidden="true">▶</span>
                </Show>
                <Show when={kind === 'audio'}>
                  <span class="tile-badge note" aria-hidden="true">♪</span>
                </Show>
              </div>
              <div class="tile-name" title={entry.name}>
                {entry.name}
              </div>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default GridView;
