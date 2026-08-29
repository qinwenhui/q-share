import { Component, For } from 'solid-js';
import { A } from '@solidjs/router';

export interface Crumb {
  label: string;
  path: string;
}

export function crumbsForPath(path: string): Crumb[] {
  const norm = !path ? '/' : path;
  const parts = norm === '/' ? [] : norm.replace(/\/+$/, '').split('/').slice(1);
  const out: Crumb[] = [{ label: 'root', path: '/' }];
  let acc = '';
  for (const p of parts) {
    acc += '/' + p;
    // Decode for display — file names may be percent-encoded.
    let label: string;
    try {
      label = decodeURIComponent(p);
    } catch {
      label = p;
    }
    out.push({ label, path: acc });
  }
  return out;
}

export const Breadcrumb: Component<{
  path: string;
  compact?: boolean;
  /** Render every crumb as a link, even the last — used on non-browse views
      (search results) where no crumb is "the current directory". */
  allLinks?: boolean;
}> = (props) => {
  const crumbs = () => crumbsForPath(props.path);
  return (
    <nav class={`breadcrumb${props.compact ? ' compact' : ''}`} aria-label="path">
      <For each={crumbs()}>
        {(c, i) => {
          const isLast = i() === crumbs().length - 1;
          const isLink = props.allLinks || !isLast;
          return (
            <>
              {i() > 0 && <span class="sep">/</span>}
              {isLink ? (
                <A href={`/browse${c.path}`} class="seg">{c.label}</A>
              ) : (
                <span class="current" title={c.path}>{c.label}</span>
              )}
            </>
          );
        }}
      </For>
    </nav>
  );
};

export default Breadcrumb;