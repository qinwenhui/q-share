// Thin client over qshare-core's REST API.
import type {
  DirListing,
  SearchOptions,
  SearchResponse,
  SortKey,
  SortOrder,
  StatResponse,
  WhoamiResponse,
} from './types';

const API_BASE = ''; // same-origin

function qs(params: Record<string, string | number | undefined | null | boolean>): string {
  const u = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v === undefined || v === null || v === '') continue;
    u.set(k, String(v));
  }
  const s = u.toString();
  return s ? `?${s}` : '';
}

async function getJson<T>(url: string, init?: RequestInit): Promise<T> {
  const r = await fetch(API_BASE + url, {
    ...init,
    headers: { Accept: 'application/json', ...(init?.headers ?? {}) },
  });
  if (!r.ok) {
    let msg = `${r.status} ${r.statusText}`;
    try {
      const j = await r.json();
      if (j?.message) msg = j.message;
    } catch {}
    throw new Error(msg);
  }
  return (await r.json()) as T;
}

export function listDir(
  path: string,
  opts: { sort?: SortKey; order?: SortOrder; offset?: number; limit?: number } = {},
): Promise<DirListing> {
  return getJson<DirListing>(
    `/api/list${qs({
      path,
      sort: opts.sort,
      order: opts.order,
      offset: opts.offset,
      limit: opts.limit,
    })}`,
  );
}

export function statFile(path: string): Promise<StatResponse> {
  return getJson<StatResponse>(`/api/stat${qs({ path })}`);
}

/** Compute a hash on-demand. Used when /api/stat's inline md5 is null
 *  (file > 64 MB). Returns the hex digest as a string. */
export async function hashFile(
  path: string,
  algo: 'md5' | 'sha256' = 'md5',
): Promise<string> {
  const r = await fetch(`${API_BASE}/api/hash${qs({ path, algo })}`);
  if (!r.ok) {
    let msg = `${r.status} ${r.statusText}`;
    try {
      const t = await r.text();
      if (t) msg = t;
    } catch {}
    throw new Error(msg);
  }
  return (await r.text()).trim();
}

export function search(
  q: string,
  path = '/',
  opts: SearchOptions = {},
): Promise<SearchResponse> {
  return getJson<SearchResponse>(
    `/api/search${qs({
      q,
      path,
      recursive: opts.recursive ? 'true' : undefined,
      depth: opts.depth,
      max_results: opts.maxResults,
      kind: opts.kind && opts.kind !== 'any' ? opts.kind : undefined,
    })}`,
  );
}

export function whoami(): Promise<WhoamiResponse> {
  return getJson<WhoamiResponse>(`/api/whoami`);
}

export function downloadUrl(path: string): string {
  return `${API_BASE}/api/raw${qs({ path })}`;
}

export function thumbUrl(path: string, w = 320): string {
  return `${API_BASE}/api/thumb${qs({ path, w })}`;
}