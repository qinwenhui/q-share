// Hand-written types mirroring qshare-core responses. Keep in sync with:
//   crates/qshare-core/src/fs/reader.rs
//   crates/qshare-core/src/routes/*.rs

export interface DirEntry {
  name: string;
  path: string; // URL path
  is_dir: boolean;
  size: number;
  modified: number; // unix seconds
  mime: string | null;
  previewable: boolean;
}

export interface DirListing {
  path: string;
  parent: string | null;
  entries: DirEntry[];
  total: number;
}

export interface StatResponse {
  path: string;
  name: string;
  is_dir: boolean;
  size: number;
  modified: number;
  /** 4-char octal mode string ("0644") on unix; "" elsewhere. */
  mode: string;
  mime: string | null;
  etag: string;
  previewable: boolean;
  /** Hex MD5 for files ≤ 64 MB. `null` for dirs and oversized files —
   *  the UI falls back to /api/hash for those. */
  md5: string | null;
}

export type PreviewKind = 'image' | 'video' | 'audio' | 'pdf' | 'text' | 'other';

export function previewKindOf(mime: string | null | undefined): PreviewKind {
  if (!mime) return 'other';
  if (mime === 'application/pdf') return 'pdf';
  if (mime.startsWith('image/')) return 'image';
  if (mime.startsWith('video/')) return 'video';
  if (mime.startsWith('audio/')) return 'audio';
  if (
    mime.startsWith('text/') ||
    mime === 'application/json' ||
    mime === 'application/javascript' ||
    mime === 'application/xml' ||
    mime === 'application/x-yaml'
  ) {
    return 'text';
  }
  return 'other';
}

/**
 * Coarse type bucket used to colour grid tiles. Folders and images get the
 * most distinctive treatment; everything else falls into a muted bucket.
 */
export type TileKind = 'folder' | 'image' | 'video' | 'audio' | 'doc' | 'other';

export function tileKindOf(entry: { is_dir: boolean; mime?: string | null; name?: string }): TileKind {
  if (entry.is_dir) return 'folder';
  const kind = previewKindOf(entry.mime);
  switch (kind) {
    case 'image': return 'image';
    case 'video': return 'video';
    case 'audio': return 'audio';
    case 'pdf':
    case 'text':  return 'doc';
    default:      return 'other';
  }
}

export interface SearchHit {
  entry: DirEntry;
  /** Path of the hit relative to the searched root (e.g. `"summer/photo.jpg"`). */
  rel_path: string;
  /** Absolute URL path of the directory containing this hit. */
  parent_dir: string;
  /** 0 for direct children, 1+ for nested hits. */
  depth: number;
}

export interface SearchResponse {
  query: string;
  root: string;
  recursive: boolean;
  total: number;
  /** True when results hit `max_results` or the walk timed out. */
  truncated: boolean;
  results: SearchHit[];
}

export interface WhoamiResponse {
  version: string;
  hostname: string;
  client_ip: string;
  shared_root: string;
  url: string;
}

export type SortKey = 'name' | 'size' | 'modified';
export type SortOrder = 'asc' | 'desc';

export interface SearchOptions {
  recursive?: boolean;
  depth?: number;
  maxResults?: number;
  kind?: 'file' | 'dir' | 'any';
}