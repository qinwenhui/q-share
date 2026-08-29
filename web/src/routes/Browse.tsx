// Browse route — directory listing view (list or grid).
//
// Path comes from the URL splat (`/browse/*path`). The view store decides
// which renderer to mount (ListView or GridView); both share the same
// selection + activation contract so keyboard navigation behaves the same.
//
// Search lives on the sidebar (SearchPanel) → /search route.

import { Component, Show, createMemo, createSignal, onMount, onCleanup } from 'solid-js';
import { useNavigate, useParams } from '@solidjs/router';
import { PinButton } from '../components/PinButton';
import { ViewSwitcher } from '../components/ViewSwitcher';
import { Toolbar } from '../components/Toolbar';
import { ListView, LIST_ROW_HEIGHT } from '../components/views/ListView';
import { GridView } from '../components/views/GridView';
import { ContextMenu } from '../components/ContextMenu';
import { DetailsModal } from '../components/DetailsModal';
import { downloadUrl } from '../api';
import { t } from '../i18n';
import {
  cycleSort,
  dirState,
  loadPath,
  normaliseUrlPath,
  parentUrlPath,
  sort,
  order,
} from '../stores/dir';
import { watchPath } from '../stores/live';
import { viewMode } from '../stores/view';
import type { DirEntry } from '../types';
import { previewKindOf } from '../types';

/** Convert `*path` splat into a leading-slash URL path. */
function splatToUrlPath(splat: string | undefined): string {
  if (!splat) return '/';
  return '/' + splat.split('/').filter(Boolean).join('/');
}

export const Browse: Component = () => {
  const params = useParams<{ path?: string }>();
  const nav = useNavigate();
  // -1 means "no selection yet" — nothing is highlighted until the user
  // first presses an arrow / j / k. Previously this was 0, which made the
  // first item look pre-selected even though nothing was clicked.
  const [selected, setSelected] = createSignal(-1);
  const [scrollEl, setScrollEl] = createSignal<HTMLDivElement | undefined>();

  // Context menu state. We keep both the entry and the click point so
  // the menu can position itself relative to the gesture. The details
  // modal opens with a separate flag so a "Details" action doesn't have
  // to share state with the menu's open/close transitions.
  const [menuEntry, setMenuEntry] = createSignal<DirEntry | null>(null);
  const [menuPos, setMenuPos] = createSignal<{ x: number; y: number }>({ x: 0, y: 0 });
  const [detailsPath, setDetailsPath] = createSignal<string | null>(null);

  // The current URL path. Solid Router re-runs this memo whenever the splat
  // changes, so navigation triggers a fresh `loadPath` + WS watch.
  const urlPath = createMemo(() => normaliseUrlPath(splatToUrlPath(params.path)));

  const entries = createMemo(() => dirState.entries?.entries ?? []);
  const listing = () => dirState.entries;

  // Re-fetch + (re)watch whenever the URL path changes.
  createMemo(() => {
    const p = urlPath();
    void loadPath(p);
    watchPath(p);
  });

  // Reset selection when the directory changes so we don't carry a stale
  // index that points past the new entries list.
  createMemo(() => {
    urlPath();
    setSelected(-1);
  });

  const activate = (entry: DirEntry) => {
    if (entry.is_dir) {
      nav('/browse' + entry.path);
      return;
    }
    const kind = previewKindOf(entry.mime);
    if (kind === 'other' || !entry.previewable) {
      window.location.href = downloadUrl(entry.path);
    } else {
      nav('/preview' + entry.path);
    }
  };

  const scrollSelectedIntoView = () => {
    const el = scrollEl();
    if (!el) return;
    if (viewMode() === 'list') {
      const target = selected() * LIST_ROW_HEIGHT;
      const visibleTop = el.scrollTop;
      const visibleBottom = visibleTop + el.clientHeight;
      if (target < visibleTop) el.scrollTop = target;
      else if (target + LIST_ROW_HEIGHT > visibleBottom) el.scrollTop = target + LIST_ROW_HEIGHT - el.clientHeight;
    } else {
      // Grid: scroll the tile element itself into view if it exists.
      const tile = el.querySelectorAll('.tile')[selected()] as HTMLElement | undefined;
      tile?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
  };

  const onKey = (e: KeyboardEvent) => {
    // Modal/menu get first dibs on keys — Escape closes them.
    if (detailsPath()) {
      if (e.key === 'Escape') { setDetailsPath(null); e.preventDefault(); }
      return;
    }
    if (menuEntry()) {
      if (e.key === 'Escape') { setMenuEntry(null); e.preventDefault(); }
      return;
    }

    const t = e.target as HTMLElement;
    if (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA') return;

    const list = entries();
    if (!list.length) return;

    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      setSelected((s) => (s < 0 ? 0 : Math.min(list.length - 1, s + 1)));
      scrollSelectedIntoView();
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      setSelected((s) => (s < 0 ? list.length - 1 : Math.max(0, s - 1)));
      scrollSelectedIntoView();
    } else if (e.key === 'Enter' || e.key === 'ArrowRight' || e.key === 'l') {
      e.preventDefault();
      // If nothing's selected yet, treat Enter as "open first item" — matches
      // the convention in Finder / Explorer where Enter on an empty selection
      // opens the first row.
      const idx = selected() < 0 ? 0 : selected();
      const entry = list[idx];
      if (entry) activate(entry);
    } else if (e.key === 'Backspace' || e.key === 'ArrowLeft' || e.key === 'h') {
      e.preventDefault();
      const parent = listing()?.parent;
      nav('/browse' + (parent ?? '/'));
    } else if (e.key === '/') {
      e.preventDefault();
      const input = document.querySelector<HTMLInputElement>('.search-input');
      input?.focus();
      input?.select();
    }
  };

  onMount(() => {
    document.addEventListener('keydown', onKey);
    onCleanup(() => document.removeEventListener('keydown', onKey));
  });

  // Up-button: visible whenever we're not at root. On mobile especially
  // there's no other obvious way to go back one level — clicking the brand
  // drops you to root, which is too coarse. Backspace/Left arrow keys
  // still work too, but a button is more discoverable.
  const goUp = () => {
    const parent = listing()?.parent;
    nav('/browse' + (parent ?? '/'));
  };
  const canGoUp = () => urlPath() !== '/';

  // Long-press / right-click handler shared by ListView and GridView.
  const onRowContext = (entry: DirEntry, x: number, y: number) => {
    setMenuEntry(entry);
    setMenuPos({ x, y });
  };

  const onMenuOpen = () => {
    const e = menuEntry();
    if (!e) return;
    if (e.is_dir) nav('/browse' + e.path);
    else activate(e);
  };
  const onMenuDownload = () => {
    const e = menuEntry();
    if (!e) return;
    window.location.href = downloadUrl(e.path);
  };
  const onMenuDetails = () => {
    const e = menuEntry();
    if (!e) return;
    setDetailsPath(e.path);
  };

  return (
    <div class="browse-pane">
      <header class="browse-header">
        <button
          type="button"
          class="up-btn"
          title={t('browse.up')}
          aria-label={t('browse.up')}
          disabled={!canGoUp()}
          onClick={goUp}
        >
          <svg viewBox="0 0 20 20" width="14" height="14" fill="none"
               stroke="currentColor" stroke-width="1.8"
               stroke-linecap="round" stroke-linejoin="round">
            <polyline points="12 5 7 10 12 15" />
          </svg>
        </button>
        <PinButton path={urlPath()} />
        <Toolbar sort={sort()} order={order()} onSort={cycleSort} />
        <div class="browse-header-spacer" />
        <ViewSwitcher />
      </header>

      <Show when={dirState.loading}>
        <div class="loading">{t('browse.loading')}</div>
      </Show>
      <Show when={dirState.error}>
        <div class="empty" style="color: var(--danger)">{dirState.error}</div>
      </Show>
      <Show when={!dirState.loading && !dirState.error}>
        <Show
          when={entries().length}
          fallback={<div class="empty">{t('browse.empty')}</div>}
        >
          <div
            class="listing"
            ref={(el) => setScrollEl(el)}
            style={{ height: 'calc(100vh - 180px)', overflow: 'auto' }}
          >
            <Show
              when={viewMode() === 'grid'}
              fallback={
                <ListView
                  entries={entries()}
                  selectedIndex={selected()}
                  onSelect={setSelected}
                  onActivate={activate}
                  onContext={onRowContext}
                />
              }
            >
              <GridView
                entries={entries()}
                selectedIndex={selected()}
                onSelect={setSelected}
                onActivate={activate}
                onContext={onRowContext}
              />
            </Show>
          </div>
        </Show>
      </Show>

      <ContextMenu
        visible={menuEntry() !== null}
        x={menuPos().x}
        y={menuPos().y}
        hasOpen
        hasDownload={!!menuEntry() && !menuEntry()!.is_dir}
        hasDetails
        onOpen={onMenuOpen}
        onDownload={onMenuDownload}
        onDetails={onMenuDetails}
        onClose={() => setMenuEntry(null)}
      />

      <DetailsModal
        path={detailsPath() ?? ''}
        visible={detailsPath() !== null}
        onClose={() => setDetailsPath(null)}
      />
    </div>
  );
};

export default Browse;
