import {
  Component,
  JSX,
  createSignal,
  createEffect,
  onCleanup,
  Show,
} from 'solid-js';
import { useLocation, A } from '@solidjs/router';
import { StatusBar } from './StatusBar';
import { Sidebar } from './Sidebar';
import { BackgroundLayer } from './BackgroundLayer';

/**
 * App shell — wraps every route with a sidebar + main content + status bar.
 */
export const AppShell: Component<{ children?: JSX.Element }> = (props) => {
  const [drawerOpen, setDrawerOpen] = createSignal(false);
  const location = useLocation();

  // Close the drawer when the route changes — on mobile the drawer is an
  // overlay, leaving it open after navigation feels broken.
  createEffect(() => {
    location.pathname; // touch to subscribe
    setDrawerOpen(false);
  });

  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') setDrawerOpen(false);
  };
  if (typeof window !== 'undefined') {
    window.addEventListener('keydown', onKey);
    onCleanup(() => window.removeEventListener('keydown', onKey));
  }

  return (
    <div class="app-shell">
      <BackgroundLayer />
      <header class="app-topbar">
        <button
          class="hamburger"
          aria-label="Open menu"
          onClick={() => setDrawerOpen((v) => !v)}
        >
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none"
               stroke="currentColor" stroke-width="2"
               stroke-linecap="round" stroke-linejoin="round">
            <line x1="3" y1="6" x2="21" y2="6"/>
            <line x1="3" y1="12" x2="21" y2="12"/>
            <line x1="3" y1="18" x2="21" y2="18"/>
          </svg>
        </button>
        <A href="/browse/" class="app-brand">q-share</A>
      </header>

      <aside class={`app-sidebar ${drawerOpen() ? 'open' : ''}`} aria-label="sidebar">
        <Sidebar />
      </aside>

      <Show when={drawerOpen()}>
        <div
          class="sidebar-scrim"
          onClick={() => setDrawerOpen(false)}
          aria-hidden="true"
        />
      </Show>

      <main class="app-main">{props.children}</main>

      <StatusBar />
    </div>
  );
};

export default AppShell;