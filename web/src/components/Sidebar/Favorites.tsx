// Favorites list shown in the sidebar.
//
// Each row shows the favorite's label, click to navigate. On hover, a small
// × appears for quick removal. Drag-to-reorder is wired via HTML5 drag&drop
// (no extra dep — the dataTransfer payload is just the row index).
//
// Empty state: shows a hint to pin something via the Pin button in the
// Browse header so the user isn't staring at a void.

import { Component, For, Show, createSignal } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import {
  favorites,
  removeFavorite,
  reorderFavorite,
  renameFavorite,
} from '../../stores/favorites';
import { t } from '../../i18n';

export const Favorites: Component = () => {
  const nav = useNavigate();
  const [editingId, setEditingId] = createSignal<string | null>(null);
  const [draftLabel, setDraftLabel] = createSignal('');
  const [dragIndex, setDragIndex] = createSignal<number | null>(null);

  const commitRename = (id: string) => {
    renameFavorite(id, draftLabel());
    setEditingId(null);
  };

  return (
    <section class="sidebar-section favorites" aria-label="favorites">
      <header class="sidebar-section-head">
        <span>{t('sidebar.favorites.title')}</span>
      </header>

      <Show
        when={favorites().length > 0}
        fallback={
          <p class="sidebar-empty">
            {t('sidebar.favorites.empty')}
          </p>
        }
      >
        <ul class="favorites-list">
          <For each={favorites()}>
            {(fav, i) => (
              <li
                class={`fav-item${dragIndex() === i() ? ' dragging' : ''}`}
                draggable={editingId() !== fav.id}
                onDragStart={(e) => {
                  if (editingId() === fav.id) {
                    e.preventDefault();
                    return;
                  }
                  setDragIndex(i());
                  e.dataTransfer?.setData('text/plain', String(i()));
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
                }}
                onDragOver={(e) => {
                  if (dragIndex() === null) return;
                  e.preventDefault();
                  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
                }}
                onDrop={(e) => {
                  e.preventDefault();
                  const from = dragIndex();
                  setDragIndex(null);
                  if (from === null || from === i()) return;
                  reorderFavorite(from, i());
                }}
                onDragEnd={() => setDragIndex(null)}
              >
                <Show
                  when={editingId() === fav.id}
                  fallback={
                    <>
                      <button
                        type="button"
                        class="fav-link"
                        title={fav.path}
                        onClick={() => nav('/browse' + fav.path)}
                        onDblClick={() => {
                          setEditingId(fav.id);
                          setDraftLabel(fav.label);
                        }}
                      >
                        <span class="fav-icon" aria-hidden="true">📌</span>
                        <span class="fav-label">{fav.label}</span>
                      </button>
                      <button
                        type="button"
                        class="fav-remove"
                        title={`remove ${fav.label}`}
                        aria-label={`remove ${fav.label}`}
                        onClick={() => removeFavorite(fav.id)}
                      >
                        ×
                      </button>
                    </>
                  }
                >
                  <input
                    type="text"
                    class="fav-rename-input"
                    value={draftLabel()}
                    onInput={(e) => setDraftLabel(e.currentTarget.value)}
                    onBlur={() => commitRename(fav.id)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') commitRename(fav.id);
                      else if (e.key === 'Escape') setEditingId(null);
                    }}
                    ref={(el) => setTimeout(() => el?.select(), 0)}
                    spellcheck={false}
                  />
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </section>
  );
};

export default Favorites;
