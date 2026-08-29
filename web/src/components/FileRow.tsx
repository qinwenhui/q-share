import { Component, Show } from 'solid-js';
import type { DirEntry } from '../types';
import { FileIcon } from './FileIcon';
import { formatBytes, formatTime } from '../utils/format';
import { thumbUrl } from '../api';
import { contextTrigger } from './ContextMenu';

interface Props {
  entry: DirEntry;
  index: number;
  selected: boolean;
  onSelect: (index: number) => void;
  onActivate: (entry: DirEntry) => void;
  /** Long-press (mobile) / right-click (desktop). Parent provides this
   *  because it owns the menu state — one menu, not N. */
  onContext?: (entry: DirEntry, x: number, y: number) => void;
}

export const FileRow: Component<Props> = (props) => {
  const onClick = () => {
    props.onSelect(props.index);
    props.onActivate(props.entry);
  };

  const isImage = () => (props.entry.mime ?? '').startsWith('image/');

  // Long-press (mobile) + right-click (desktop). The trigger function
  // returns a spread of event handlers we apply to the row's outer div.
  const triggerProps = () =>
    props.onContext
      ? contextTrigger((x, y) => props.onContext!(props.entry, x, y))
      : {};

  return (
    <div
      class={`row ${props.selected ? 'selected' : ''}`}
      onClick={onClick}
      role="row"
      aria-selected={props.selected}
      {...triggerProps()}
    >
      <div class="name">
        <span class="icon">
          <Show when={!props.entry.is_dir && isImage()} fallback={
            <FileIcon isDir={props.entry.is_dir} mime={props.entry.mime} name={props.entry.name} />
          }>
            <img
              class="thumb"
              src={thumbUrl(props.entry.path, 48)}
              loading="lazy"
              alt=""
              onError={(e) => {
                // Fall back to icon if thumbnail fails.
                (e.currentTarget as HTMLImageElement).style.display = 'none';
              }}
            />
          </Show>
        </span>
        <span class="label" title={props.entry.name}>
          {props.entry.name}
        </span>
      </div>
      <div class="size">{props.entry.is_dir ? '—' : formatBytes(props.entry.size)}</div>
      <div class="modified">{formatTime(props.entry.modified)}</div>
    </div>
  );
};
