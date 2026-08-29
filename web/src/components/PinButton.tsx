// Pin/unpin button — toggles the current path in the favorites list.
//
// Lives in the Browse header (next to the breadcrumb). Click to pin, click
// again to unpin. Disabled on `/` (root) since pinning root is pointless —
// the favorite would always be "go to root", which is what the brand link
// in the topbar already does.
//
// The "favorites" signal updates reactively so the sidebar list updates
// without any extra wiring.

import { Component, Show } from 'solid-js';
import {
  favorites,
  addFavorite,
  removeFavorite,
} from '../stores/favorites';
import { t } from '../i18n';

interface Props {
  path: string;
}

export const PinButton: Component<Props> = (props) => {
  const isPinned = () => favorites().some((f) => f.path === props.path);
  const disabled = () => !props.path || props.path === '/' || props.path === '';

  const toggle = () => {
    if (disabled()) return;
    if (isPinned()) {
      const fav = favorites().find((f) => f.path === props.path);
      if (fav) removeFavorite(fav.id);
    } else {
      addFavorite(props.path);
    }
  };

  return (
    <button
      type="button"
      class={`pin-btn${isPinned() ? ' pinned' : ''}`}
      title={
        disabled()
          ? t('sidebar.favorites.nothing')
          : isPinned()
          ? t('sidebar.favorites.unpin_title')
          : t('sidebar.favorites.pin_title')
      }
      aria-label={isPinned() ? t('sidebar.favorites.unpin_title') : t('sidebar.favorites.pin_title')}
      aria-pressed={isPinned()}
      disabled={disabled()}
      onClick={toggle}
    >
      <Show
        when={isPinned()}
        fallback={
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none"
               stroke="currentColor" stroke-width="2"
               stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 17v5" />
            <path d="M9 10.76V6h6v4.76l3 4.24H6l3-4.24z" />
          </svg>
        }
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
          <path d="M9 10.76V6h6v4.76l3 4.24H6l3-4.24zM12 17v5" stroke="currentColor" stroke-width="2" fill="none" />
        </svg>
      </Show>
      <span class="pin-label">{isPinned() ? t('sidebar.favorites.added') : t('sidebar.favorites.add')}</span>
    </button>
  );
};

export default PinButton;
