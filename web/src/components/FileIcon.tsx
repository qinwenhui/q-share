// File icon — typed colour-coded body for files, matching folder for dirs.
//
// Categories are matched by extension first (longer prefixes win because
// the lookup is exact-string), then by mime prefix as a fallback. Unknown
// types fall back to the file-extension name shown inside the body so the
// user can still tell files apart at a glance.

import { Show, type Component } from 'solid-js';

interface Props {
  isDir: boolean;
  mime?: string | null;
  name: string;
  /** "lg" renders the bigger grid tile variant (with the file body and
   *  a folded corner); "sm" renders a flat 20×20 mark for list rows. */
  size?: 'sm' | 'lg';
}

// ────────────────────────────────────────────────────────────────────
// Folder body — uses an explicit accent fill instead of `currentColor`
// so the icon is always visible regardless of the parent text colour.
// `currentColor` was prone to matching theme tokens that resolved to
// something close to the row background, making the icon effectively
// invisible for most directories.
// ────────────────────────────────────────────────────────────────────

function FolderIcon(props: { size?: 'sm' | 'lg' }) {
  const sz = props.size === 'lg' ? 36 : 20;
  return (
    <svg
      width={sz}
      height={sz}
      viewBox="0 0 20 20"
      aria-hidden="true"
    >
      <path
        d="M2 5a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5z"
        fill="var(--accent)"
      />
    </svg>
  );
}

// ────────────────────────────────────────────────────────────────────
// Category colours and labels.
// ────────────────────────────────────────────────────────────────────

type Category =
  | 'image' | 'video' | 'audio'
  | 'doc'   // pdf
  | 'code-rust' | 'code-python' | 'code-go' | 'code-js' | 'code-ts'
  | 'code-java' | 'code-c' | 'code-ruby' | 'code-php' | 'code-shell'
  | 'code-html' | 'code-css' | 'code-vue' | 'code-svelte' | 'code-lua' | 'code-swift'
  | 'markup-md' | 'markup-txt' | 'data-json' | 'data-yaml' | 'data-xml' | 'data-csv'
  | 'config' | 'env'
  | 'archive' | 'disk' | 'font'
  | 'database' | 'spreadsheet'
  | 'generic';

const CATEGORY_COLOR: Record<Category, string> = {
  image:           '#51cf66',
  video:           '#9775fa',
  audio:           '#ff9f43',
  doc:             '#ff6b6b',
  'code-rust':     '#ce422b',
  'code-python':   '#3776ab',
  'code-go':       '#00add8',
  'code-js':       '#f7df1e',
  'code-ts':       '#3178c6',
  'code-java':     '#ea2d2e',
  'code-c':        '#5c6bc0',
  'code-ruby':     '#cc342d',
  'code-php':      '#8993be',
  'code-shell':    '#4eaa25',
  'code-html':     '#e34c26',
  'code-css':      '#264de4',
  'code-vue':      '#42b883',
  'code-svelte':   '#ff3e00',
  'code-lua':      '#000080',
  'code-swift':    '#ff8c00',
  'markup-md':     '#9aa3b2',
  'markup-txt':    '#9aa3b2',
  'data-json':     '#9aa3b2',
  'data-yaml':     '#cb171e',
  'data-xml':      '#9aa3b2',
  'data-csv':      '#217346',
  config:          '#5b6478',
  env:             '#ecd75f',
  archive:         '#845ef7',
  disk:            '#868e96',
  font:            '#20c997',
  database:        '#339af0',
  spreadsheet:     '#37b24d',
  generic:         '#868e96',
};

const CATEGORY_LABEL: Record<Category, string> = {
  image:           'IMG',
  video:           'VID',
  audio:           'AUD',
  doc:             'PDF',
  'code-rust':     'RS',
  'code-python':   'PY',
  'code-go':       'GO',
  'code-js':       'JS',
  'code-ts':       'TS',
  'code-java':     'JV',
  'code-c':        'C',
  'code-ruby':     'RB',
  'code-php':      'PHP',
  'code-shell':    'SH',
  'code-html':     'HTM',
  'code-css':      'CSS',
  'code-vue':      'VUE',
  'code-svelte':   'SVT',
  'code-lua':      'LUA',
  'code-swift':    'SW',
  'markup-md':     'MD',
  'markup-txt':    'TXT',
  'data-json':     'JSON',
  'data-yaml':     'YML',
  'data-xml':      'XML',
  'data-csv':      'CSV',
  config:          'CFG',
  env:             'ENV',
  archive:         'ZIP',
  disk:            'ISO',
  font:            'FONT',
  database:        'DB',
  spreadsheet:     'XLS',
  generic:         'FILE',
};

const EXT_TABLE: { ext: string; cat: Category }[] = [
  // Code
  { ext: 'rs',     cat: 'code-rust' },
  { ext: 'py',     cat: 'code-python' },
  { ext: 'go',     cat: 'code-go' },
  { ext: 'js',     cat: 'code-js' },
  { ext: 'mjs',    cat: 'code-js' },
  { ext: 'cjs',    cat: 'code-js' },
  { ext: 'jsx',    cat: 'code-js' },
  { ext: 'ts',     cat: 'code-ts' },
  { ext: 'tsx',    cat: 'code-ts' },
  { ext: 'mts',    cat: 'code-ts' },
  { ext: 'cts',    cat: 'code-ts' },
  { ext: 'java',   cat: 'code-java' },
  { ext: 'kt',     cat: 'code-java' },
  { ext: 'scala',  cat: 'code-java' },
  { ext: 'c',      cat: 'code-c' },
  { ext: 'h',      cat: 'code-c' },
  { ext: 'cpp',    cat: 'code-c' },
  { ext: 'cc',     cat: 'code-c' },
  { ext: 'cxx',    cat: 'code-c' },
  { ext: 'hpp',    cat: 'code-c' },
  { ext: 'rb',     cat: 'code-ruby' },
  { ext: 'php',    cat: 'code-php' },
  { ext: 'sh',     cat: 'code-shell' },
  { ext: 'bash',   cat: 'code-shell' },
  { ext: 'zsh',    cat: 'code-shell' },
  { ext: 'fish',   cat: 'code-shell' },
  { ext: 'ps1',    cat: 'code-shell' },
  { ext: 'bat',    cat: 'code-shell' },
  { ext: 'cmd',    cat: 'code-shell' },
  { ext: 'html',   cat: 'code-html' },
  { ext: 'htm',    cat: 'code-html' },
  { ext: 'css',    cat: 'code-css' },
  { ext: 'scss',   cat: 'code-css' },
  { ext: 'sass',   cat: 'code-css' },
  { ext: 'less',   cat: 'code-css' },
  { ext: 'vue',    cat: 'code-vue' },
  { ext: 'svelte', cat: 'code-svelte' },
  { ext: 'lua',    cat: 'code-lua' },
  { ext: 'swift',  cat: 'code-swift' },
  // Markup / data
  { ext: 'md',     cat: 'markup-md' },
  { ext: 'mdx',    cat: 'markup-md' },
  { ext: 'txt',    cat: 'markup-txt' },
  { ext: 'log',    cat: 'markup-txt' },
  { ext: 'json',   cat: 'data-json' },
  { ext: 'json5',  cat: 'data-json' },
  { ext: 'jsonc',  cat: 'data-json' },
  { ext: 'yaml',   cat: 'data-yaml' },
  { ext: 'yml',    cat: 'data-yaml' },
  { ext: 'xml',    cat: 'data-xml' },
  { ext: 'csv',    cat: 'data-csv' },
  // Config / env
  { ext: 'toml',   cat: 'config' },
  { ext: 'ini',    cat: 'config' },
  { ext: 'conf',   cat: 'config' },
  { ext: 'cfg',    cat: 'config' },
  { ext: 'lock',   cat: 'config' },
  { ext: 'env',    cat: 'env' },
  // Archives
  { ext: 'zip',    cat: 'archive' },
  { ext: 'tar',    cat: 'archive' },
  { ext: 'gz',     cat: 'archive' },
  { ext: 'tgz',    cat: 'archive' },
  { ext: 'bz2',    cat: 'archive' },
  { ext: 'xz',     cat: 'archive' },
  { ext: '7z',     cat: 'archive' },
  { ext: 'rar',    cat: 'archive' },
  { ext: 'jar',    cat: 'archive' },
  { ext: 'war',    cat: 'archive' },
  { ext: 'whl',    cat: 'archive' },
  // Disk / image
  { ext: 'iso',    cat: 'disk' },
  { ext: 'dmg',    cat: 'disk' },
  { ext: 'img',    cat: 'disk' },
  { ext: 'vmdk',   cat: 'disk' },
  { ext: 'qcow2',  cat: 'disk' },
  // Fonts
  { ext: 'ttf',    cat: 'font' },
  { ext: 'otf',    cat: 'font' },
  { ext: 'woff',   cat: 'font' },
  { ext: 'woff2',  cat: 'font' },
  { ext: 'eot',    cat: 'font' },
  // Database / spreadsheet
  { ext: 'sql',    cat: 'database' },
  { ext: 'sqlite', cat: 'database' },
  { ext: 'db',     cat: 'database' },
  { ext: 'xlsx',   cat: 'spreadsheet' },
  { ext: 'xls',    cat: 'spreadsheet' },
  { ext: 'ods',    cat: 'spreadsheet' },
];

function extOf(name: string): string {
  const dot = name.lastIndexOf('.');
  if (dot < 0 || dot === name.length - 1) return '';
  return name.slice(dot + 1).toLowerCase();
}

function categoryFor(name: string, mime: string | null | undefined): Category {
  const m = mime ?? '';
  if (m === 'application/pdf') return 'doc';
  if (m.startsWith('image/')) return 'image';
  if (m.startsWith('video/')) return 'video';
  if (m.startsWith('audio/')) return 'audio';

  const ext = extOf(name);
  if (ext) {
    const hit = EXT_TABLE.find((e) => e.ext === ext);
    if (hit) return hit.cat;
  }
  if (m.startsWith('text/')) {
    if (m === 'text/markdown') return 'markup-md';
    return 'markup-txt';
  }
  if (m.includes('zip') || m.includes('compressed') || m.includes('tar')) return 'archive';
  if (m.includes('font')) return 'font';
  if (m.includes('xml'))  return 'data-xml';
  if (m.includes('json')) return 'data-json';
  if (m.includes('yaml')) return 'data-yaml';
  return 'generic';
}

function Glyph(props: { cat: Category; size?: 'sm' | 'lg' }) {
  const sz = props.size === 'lg' ? 14 : 9;
  const label = CATEGORY_LABEL[props.cat];
  const labelColor: 'light' | 'dark' =
    props.cat === 'code-js' || props.cat === 'env' ? 'dark' : 'light';

  return (
    <Show
      when={props.size === 'lg' && (
        props.cat === 'image' || props.cat === 'video' ||
        props.cat === 'audio' || props.cat === 'doc'
      )}
      fallback={
        <text
          x="10" y="13"
          text-anchor="middle"
          font-size={String(sz * 0.65)}
          font-weight="600"
          font-family="-apple-system, system-ui, sans-serif"
          fill={labelColor === 'light' ? 'white' : '#1a1d23'}
        >
          {label}
        </text>
      }
    >
      <Show when={props.cat === 'image'}>
        <g fill="none" stroke="white" stroke-width="1.2"
           stroke-linecap="round" stroke-linejoin="round">
          <rect x="5" y="6" width="10" height="8" rx="1" />
          <circle cx="8" cy="9" r="1" />
          <path d="m5 12 3-3 3 3 2-2 2 2" />
        </g>
      </Show>
      <Show when={props.cat === 'video'}>
        <g fill="none" stroke="white" stroke-width="1.2"
           stroke-linecap="round" stroke-linejoin="round">
          <rect x="4" y="6" width="12" height="8" rx="1" />
          <path d="m9 8 4 2-4 2z" fill="white" />
        </g>
      </Show>
      <Show when={props.cat === 'audio'}>
        <g fill="none" stroke="white" stroke-width="1.2"
           stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 4v9a2.5 2.5 0 1 1-2-2.4" />
          <path d="M9 4l6-1v8" />
        </g>
      </Show>
      <Show when={props.cat === 'doc'}>
        <text
          x="10" y="13"
          text-anchor="middle"
          font-size="7"
          font-weight="700"
          font-family="-apple-system, system-ui, sans-serif"
          fill="white"
        >PDF</text>
      </Show>
    </Show>
  );
}

function TypedFile(props: { cat: Category; size?: 'sm' | 'lg' }) {
  const color = CATEGORY_COLOR[props.cat];
  return (
    <svg
      width={props.size === 'lg' ? 36 : 20}
      height={props.size === 'lg' ? 36 : 20}
      viewBox="0 0 20 20"
      aria-hidden="true"
    >
      <path d="M4 2h8l4 4v12H4z" fill={color} />
      <path d="M12 2v4h4" fill="rgba(0,0,0,0.18)" />
      <Glyph cat={props.cat} size={props.size} />
    </svg>
  );
}

export const FileIcon: Component<Props> = (props) => {
  return (
    <Show when={props.isDir} fallback={
      <TypedFile
        cat={categoryFor(props.name, props.mime)}
        size={props.size ?? 'sm'}
      />
    }>
      <FolderIcon size={props.size ?? 'sm'} />
    </Show>
  );
};

export default FileIcon;