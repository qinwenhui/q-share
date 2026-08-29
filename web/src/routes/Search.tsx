// Search results route — `/search?q=...&path=...&recursive=...`
//
// Renders results fetched from /api/search. Results from a recursive search
// are grouped by their parent directory so the user can see "where in the
// tree" each match lives at a glance. Single-directory results render as
// a flat list (no group headers).
//
// URL is the source of truth — back/forward navigation re-runs the query
// automatically. We don't persist the query in component state.

import { Component, Show, For, createResource, createMemo } from 'solid-js';
import { useSearchParams, useNavigate } from '@solidjs/router';
import { search as searchApi } from '../api';
import { formatBytes, formatTime } from '../utils/format';
import { previewKindOf } from '../types';

interface Group {
  parentDir: string;
  hits: ReturnType<typeof toHits>;
}

function toHits(r: Awaited<ReturnType<typeof searchApi>>['results']) {
  return r;
}

export const Search: Component = () => {
  const [params] = useSearchParams();
  const nav = useNavigate();

  const query = () => (typeof params.q === 'string' ? params.q : '');
  const path = () => (typeof params.path === 'string' ? params.path : '/');
  const recursive = () => params.recursive === 'true';
  const depth = () => (typeof params.depth === 'string' ? Number(params.depth) : 5);

  const [result] = createResource(
    () => ({
      q: query(),
      path: path(),
      recursive: recursive(),
      depth: depth(),
    }),
    async (p) => {
      if (!p.q) return null;
      return searchApi(p.q, p.path, {
        recursive: p.recursive,
        depth: p.depth,
        kind: 'any',
      });
    },
  );

  // Group by parent_dir when the search was recursive AND actually
  // produced hits across multiple dirs. Single-dir results render flat.
  const grouped = createMemo<Group[]>(() => {
    const r = result();
    if (!r) return [];
    if (!recursive()) return [{ parentDir: path(), hits: r.results }];
    const map = new Map<string, ReturnType<typeof toHits>>();
    for (const hit of r.results) {
      const dir = hit.parent_dir || '/';
      if (!map.has(dir)) map.set(dir, []);
      map.get(dir)!.push(hit);
    }
    return [...map.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([parentDir, hits]) => ({ parentDir, hits }));
  });

  const activate = (path: string, isDir: boolean, mime?: string | null, previewable = false) => {
    if (isDir) {
      nav('/browse' + path);
      return;
    }
    const k = previewKindOf(mime);
    if (k === 'other' || !previewable) {
      window.location.href = `/api/raw?path=${encodeURIComponent(path)}`;
    } else {
      nav('/preview' + path);
    }
  };

  return (
    <div class="browse-pane search-route">
      <header class="browse-header">
        <button
          type="button"
          class="search-back"
          onClick={() => nav('/browse' + path())}
          title={`open ${path()} in the file browser`}
        >
          ← back to <code>{path()}</code>
        </button>
        <div class="search-route-summary">
          <strong>{query() || '—'}</strong>
          <span class="search-route-meta">
            in <code>{path()}</code>
            {recursive() ? ` · recursive · depth ${depth()}` : ''}
          </span>
        </div>
      </header>

      <Show when={result.loading}>
        <div class="loading">searching…</div>
      </Show>

      <Show when={result.error}>
        <div class="empty" style="color: var(--danger)">
          {String(result.error)}
        </div>
      </Show>

      <Show when={result() && !result.loading}>
        <Show
          when={result()!.results.length > 0}
          fallback={
            <div class="empty">
              no results for <strong>{query()}</strong>
            </div>
          }
        >
          <div class="search-results">
            <For each={grouped()}>
              {(group) => (
                <>
                  <Show when={recursive()}>
                    <button
                      type="button"
                      class="search-group-head"
                      onClick={() => nav('/browse' + group.parentDir)}
                      title={`open ${group.parentDir}`}
                    >
                      <span class="search-group-icon">📁</span>
                      <span class="search-group-path">{group.parentDir}</span>
                      <span class="search-group-count">{group.hits.length}</span>
                    </button>
                  </Show>
                  <For each={group.hits}>
                    {(hit) => (
                      <div
                        class="hit"
                        onClick={() =>
                          activate(
                            hit.entry.path,
                            hit.entry.is_dir,
                            hit.entry.mime,
                            hit.entry.previewable,
                          )
                        }
                      >
                        <Show when={recursive()}>
                          <span class="hit-depth">d{hit.depth}</span>
                        </Show>
                        <span class="hit-kind">
                          {hit.entry.is_dir ? '📁' : formatBytes(hit.entry.size)}
                        </span>
                        <span class="hit-name">{hit.entry.name}</span>
                        <Show when={recursive()}>
                          <span class="hit-rel">{hit.rel_path}</span>
                        </Show>
                        <span class="hit-time">{formatTime(hit.entry.modified)}</span>
                      </div>
                    )}
                  </For>
                </>
              )}
            </For>

            <Show when={result()?.truncated}>
              <div class="search-truncated">
                results truncated — refine your query (max {result()?.results.length} shown)
              </div>
            </Show>
          </div>
        </Show>
      </Show>
    </div>
  );
};

export default Search;
