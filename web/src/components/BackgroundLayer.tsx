// Fixed full-viewport background behind the app shell. One instance lives at
// the root of AppShell and renders the theme store's `background` (none, a
// URL, or an uploaded data-URL) with the user's chosen blur + overlay.
//
// It's an <img> rather than a CSS background-image so failures show a broken
// image (debuggable in devtools) and users can right-save-as their uploads.
// The layer sits at z-index -1 with pointer-events: none so it never
// intercepts clicks.

import { Component, Show, createMemo } from 'solid-js';
import { currentTheme } from '../stores/theme';

export const BackgroundLayer: Component = () => {
  const bg = () => currentTheme().background;

  const imageSrc = createMemo(() => {
    const b = bg();
    if (b.type === 'url' && b.url) return b.url;
    if (b.type === 'data' && b.dataUrl) return b.dataUrl;
    return null;
  });

  return (
    <Show when={imageSrc()}>
      <div class="bg-layer" aria-hidden="true">
        <img
          class="bg-img"
          src={imageSrc()!}
          alt=""
          style={{
            filter: `blur(${bg().blur}px)`,
          }}
        />
        <div
          class="bg-veil"
          style={{ 'background-color': `rgba(0, 0, 0, ${bg().overlay})` }}
        />
      </div>
    </Show>
  );
};

export default BackgroundLayer;
