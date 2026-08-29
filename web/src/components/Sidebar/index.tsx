// Sidebar — vertical column on the left containing the navigation aids:
// path input + breadcrumb, favorites, search, and the settings button.
//
// Order matters: PathBar + Breadcrumb first (the "where am I / go there"
// controls), Favorites next (jump to a pinned place), Search below (the
// "find-a-thing" escape hatch), Settings at the very bottom (least-used).

import { Component } from 'solid-js';
import { useLocation } from '@solidjs/router';
import { PathBar } from './PathBar';
import { Breadcrumb } from '../Breadcrumb';
import { Favorites } from './Favorites';
import { SearchPanel } from './SearchPanel';
import { SettingsButton } from './SettingsButton';

export const Sidebar: Component = () => {
  const location = useLocation();

  const isSearch = () => location.pathname.startsWith('/search');

  // Derive the current URL path the same way PathBar does, so the breadcrumb
  // shows the right crumbs on /browse and /preview alike. On /search there is
  // no directory being viewed, so we fall back to the scope the search ran in.
  const currentPath = (): string => {
    const m = location.pathname.match(/^\/browse\/(.*)$/);
    if (m) return '/' + (m[1] ?? '');
    const p = location.pathname.match(/^\/preview\/(.*)$/);
    if (p) return '/' + (decodeURIComponent(p[1] ?? '') || '');
    if (isSearch()) {
      return new URLSearchParams(location.search).get('path') || '/';
    }
    return '/';
  };

  return (
    <div class="sidebar-stack">
      <PathBar />
      {/* On /search no crumb is "the current directory" (the view is a
          results list), so every crumb — including root — is a link back
          into the file browser. */}
      <Breadcrumb path={currentPath()} allLinks={isSearch()} />
      <Favorites />
      <SearchPanel />
      <div class="sidebar-spacer" />
      <SettingsButton />
    </div>
  );
};

export default Sidebar;
