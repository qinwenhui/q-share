// Search panel in the sidebar.
//
// Submitting the form navigates to /search?q=...&path=...&recursive=...
// where the Search route renders the results. We don't fetch results here —
// the route owns that, so back/forward navigation works correctly and the
// results page can deep-link.
//
// "Recursive" toggle controls whether the search descends into
// subdirectories. "From root" lets users search the entire shared tree
// rather than just the current directory.

import { Component, createSignal, Show } from 'solid-js';
import { useLocation, useNavigate } from '@solidjs/router';
import { t } from '../../i18n';

interface Props {
  /** Show the "from root" toggle. Hidden on the Search route itself to
   *  reduce clutter once results are visible. */
  showScope?: boolean;
}

export const SearchPanel: Component<Props> = (props) => {
  const nav = useNavigate();
  const location = useLocation();

  // Pre-fill from ?q= if we're already on /search.
  const initialQ = () => {
    const m = location.pathname === '/search';
    if (!m) return '';
    const sp = new URLSearchParams(location.search);
    return sp.get('q') ?? '';
  };

  const [q, setQ] = createSignal(initialQ());
  // Recursive defaults to OFF — most users want "find the file in this
  // folder" first, not a deep tree walk. Toggling it on for the rare
  // "where is foo.jpg anywhere?" case.
  const [recursive, setRecursive] = createSignal(false);
  const [fromRoot, setFromRoot] = createSignal(false);

  const submit = (e?: Event) => {
    e?.preventDefault();
    const query = q().trim();
    if (!query) return;
    const params = new URLSearchParams();
    params.set('q', query);
    if (fromRoot()) params.set('path', '/');
    if (recursive()) params.set('recursive', 'true');
    nav(`/search?${params.toString()}`);
  };

  return (
    <section class="sidebar-section search-panel" aria-label="search">
      <header class="sidebar-section-head">
        <span>{t('sidebar.search.placeholder').replace('…', '')}</span>
        <div class="search-toggles">
          <button
            type="button"
            class={`search-toggle${recursive() ? ' on' : ''}`}
            title={recursive() ? t('sidebar.search.recursive') : ''}
            aria-pressed={recursive()}
            onClick={() => setRecursive((v) => !v)}
          >
            ↘︎
          </button>
          <Show when={props.showScope !== false}>
            <button
              type="button"
              class={`search-toggle${fromRoot() ? ' on' : ''}`}
              title={fromRoot() ? t('sidebar.search.scope.root') : t('sidebar.search.scope.here')}
              aria-pressed={fromRoot()}
              onClick={() => setFromRoot((v) => !v)}
            >
              ↥
            </button>
          </Show>
        </div>
      </header>

      <form class="search-form" onSubmit={submit}>
        <input
          type="search"
          class="search-input"
          placeholder={t('sidebar.search.placeholder')}
          value={q()}
          onInput={(e) => setQ(e.currentTarget.value)}
          spellcheck={false}
          autocomplete="off"
          aria-label="search query"
        />
      </form>
    </section>
  );
};

export default SearchPanel;
