// Floating context menu — right-click (desktop) or long-press (mobile).
// Two actions for now: download + details; directories only get
// "open" / "details" (no download).
//
// The host positions it from the event's (x, y) and toggles `visible` to
// open/close; we dismiss on outside-click, scroll, Escape, or window blur.

import { Component, Show, createSignal, onMount, onCleanup } from 'solid-js';
import { Portal } from 'solid-js/web';
import { t } from '../i18n';

interface Props {
  visible: boolean;
  x: number;
  y: number;
  /** When true, include the "details" action. False for entries where
   *  details don't add value (e.g. directories in some views). */
  hasDetails?: boolean;
  hasDownload?: boolean;
  hasOpen?: boolean;
  onOpen?: () => void;
  onDownload?: () => void;
  onDetails?: () => void;
  onClose: () => void;
}

export const ContextMenu: Component<Props> = (props) => {
  // Clamp position so the menu doesn't fly off the right/bottom edge.
  const pos = () => {
    const MENU_W = 180;
    const MENU_H = 120;
    const maxX = window.innerWidth - MENU_W - 8;
    const maxY = window.innerHeight - MENU_H - 8;
    return {
      left: Math.max(8, Math.min(props.x, maxX)) + 'px',
      top: Math.max(8, Math.min(props.y, maxY)) + 'px',
    };
  };

  const onDocClick = (e: MouseEvent) => {
    if (!props.visible) return;
    const t = e.target as HTMLElement;
    if (t.closest('.context-menu')) return;
    props.onClose();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && props.visible) props.onClose();
  };
  const onScroll = () => { if (props.visible) props.onClose(); };

  onMount(() => {
    document.addEventListener('mousedown', onDocClick);
    document.addEventListener('keydown', onKey);
    window.addEventListener('scroll', onScroll, true);
    window.addEventListener('blur', props.onClose);
    onCleanup(() => {
      document.removeEventListener('mousedown', onDocClick);
      document.removeEventListener('keydown', onKey);
      window.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('blur', props.onClose);
    });
  });

  return (
    <Show when={props.visible}>
      <Portal>
        <div
          class="context-menu"
          role="menu"
          style={{ left: pos().left, top: pos().top }}
          onContextMenu={(e) => e.preventDefault()}
        >
          <Show when={props.hasOpen !== false}>
            <button
              type="button"
              role="menuitem"
              onClick={() => { props.onOpen?.(); props.onClose(); }}
            >
              {t('menu.open')}
            </button>
          </Show>
          <Show when={props.hasDownload}>
            <button
              type="button"
              role="menuitem"
              onClick={() => { props.onDownload?.(); props.onClose(); }}
            >
              {t('menu.download')}
            </button>
          </Show>
          <Show when={props.hasDetails}>
            <button
              type="button"
              role="menuitem"
              onClick={() => { props.onDetails?.(); props.onClose(); }}
            >
              {t('menu.details')}
            </button>
          </Show>
        </div>
      </Portal>
    </Show>
  );
};

/** Trigger detection: long-press for touch, right-click for mouse.
 *  Returns a `trigger` function suitable for spreading onto a div. */
export function contextTrigger(onTrigger: (x: number, y: number) => void) {
  let timer: number | null = null;
  let startX = 0;
  let startY = 0;

  const onTouchStart = (e: TouchEvent) => {
    if (e.touches.length !== 1) return;
    const t = e.touches[0];
    startX = t.clientX;
    startY = t.clientY;
    timer = window.setTimeout(() => {
      onTrigger(startX, startY);
      timer = null;
    }, 480);
  };
  const onTouchMove = () => {
    // Cancel the long-press if the user drags — otherwise scrolling the
    // page would constantly pop the menu.
    if (timer !== null) {
      window.clearTimeout(timer);
      timer = null;
    }
  };
  const onTouchEnd = () => {
    if (timer !== null) {
      window.clearTimeout(timer);
      timer = null;
    }
  };
  const onContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    onTrigger(e.clientX, e.clientY);
  };

  return {
    onTouchStart,
    onTouchMove,
    onTouchEnd,
    onContextMenu,
  };
}

export default ContextMenu;
