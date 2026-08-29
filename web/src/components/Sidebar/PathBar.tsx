// Editable path input at the top of the sidebar. Type any path under the
// shared root and jump to it via Enter.
//
// Validation runs locally first (must start with `/`, no `..` segments, no
// control chars), but the backend re-validates through the sandbox — a path
// that passes here can still 404 if it doesn't exist on disk.
//
// Works outside `/browse/` too: the current path comes from the URL via
// useLocation, so it functions on /preview, /search, etc.

import { Component, createSignal, createEffect, Show, on } from 'solid-js';
import { useLocation, useNavigate } from '@solidjs/router';
import { isValidUrlPath, normaliseUrlPath } from '../../stores/dir';

export const PathBar: Component = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const [value, setValue] = createSignal('');
  const [editing, setEditing] = createSignal(false);
  const [invalid, setInvalid] = createSignal<string | null>(null);

  // Derive the current URL path from `/browse/*path` (and friends).
  const currentPath = (): string => {
    const m = location.pathname.match(/^\/browse\/(.*)$/);
    if (m) return '/' + (m[1] ?? '');
    const p = location.pathname.match(/^\/preview\/(.*)$/);
    if (p) return '/' + (decodeURIComponent(p[1] ?? '') || '');
    // /search has no directory of its own — reflect the scope it searched
    // (defaults to root) so the bar isn't a dead "…" that suggests we're at
    // root when we're actually on a results page.
    if (location.pathname.startsWith('/search')) {
      return new URLSearchParams(location.search).get('path') || '/';
    }
    return '/';
  };

  // Sync the displayed value with the current path whenever the route changes
  // (but only while we're not actively editing — otherwise we'd trample the
  // user's keystrokes).
  createEffect(
    on(
      // Search params matter too — on /search the "current" path comes from
      // ?path=, and re-searching in a different dir must update the bar.
      () => location.pathname + location.search,
      () => {
        if (!editing()) setValue(currentPath());
      },
    ),
  );

  if (value() === '') setValue(currentPath());

  let inputRef: HTMLInputElement | undefined;

  const onFocus = () => {
    setEditing(true);
    // Defer select-all so the click that focused us doesn't collapse the
    // selection.
    setTimeout(() => inputRef?.select(), 0);
  };

  const commit = () => {
    const raw = value().trim();
    if (!raw) {
      setValue(currentPath());
      setEditing(false);
      setInvalid(null);
      inputRef?.blur();
      return;
    }
    const norm = normaliseUrlPath(raw);
    if (!isValidUrlPath(norm)) {
      setInvalid('path must start with / and contain no ".."');
      return;
    }
    setInvalid(null);
    setEditing(false);
    inputRef?.blur();
    // On /browse the current path *is* the real location, so re-submitting it
    // is a harmless no-op. Everywhere else (search results, preview) the path
    // bar is an escape hatch: Enter must actually leave that view and open
    // the directory — otherwise users stuck on search have no way back.
    const onBrowse = location.pathname.startsWith('/browse');
    if (onBrowse && norm === currentPath()) {
      setValue(norm);
    } else {
      navigate('/browse' + norm);
    }
  };

  const cancel = () => {
    setValue(currentPath());
    setEditing(false);
    setInvalid(null);
    inputRef?.blur();
  };

  return (
    <div class={`path-bar${invalid() ? ' invalid' : ''}`}>
      <input
        ref={(el) => (inputRef = el)}
        type="text"
        class="path-bar-input"
        value={value()}
        spellcheck={false}
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        placeholder="/"
        aria-label="path"
        title={invalid() ?? currentPath()}
        onInput={(e) => {
          setValue(e.currentTarget.value);
          if (invalid()) setInvalid(null);
        }}
        onFocus={onFocus}
        onBlur={() => {
          // If user clicks away without pressing Enter, revert so we don't
          // leave a half-edited value lying around.
          if (editing()) cancel();
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            commit();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            cancel();
          }
        }}
      />
      <Show when={invalid()}>
        <div class="path-bar-error" role="alert">{invalid()}</div>
      </Show>
    </div>
  );
};

export default PathBar;
