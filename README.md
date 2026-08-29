# q-share

**English** · [**中文**](README.zh-CN.md)

[![CI](https://github.com/qinwenhui/q-share/actions/workflows/ci.yml/badge.svg)](https://github.com/qinwenhui/q-share/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> **Zero-config LAN file sharing** — pick a folder, get a URL + QR code, share with anyone on the network.

`q-share` turns any local directory into a modern, fast, browser-based file browser that anyone on your LAN can open. No accounts, no uploads, no cloud — just browse, preview, search and download.

Use the native GUI, or run the headless CLI from a terminal:

```
$ qshare-cli -r ~/Videos

  q-share  0.1.1
  ─────────────────────────────────────────────────
  shared  /Users/me/Videos
  url     http://192.168.1.42:8888
  qr      ▆▆▆▆▆▆ ▆▆▆  ▆▆▆▆▆ ▆▆▆▆▆▆
          ▆  ▆▆▆ ▆▆▆▆ ▆  ▆   ▆  ▆
          ▆▆ ▆▆▆ ▆ ▆▆ ▆▆▆▆▆ ▆▆▆▆
  ─────────────────────────────────────────────────
  active     3
  bytes      12.4 MB
  errors     0
```

Or launch the native GUI — a single window: pick a directory, hit **Start**, copy the URL or scan the QR. Live counters tick up as people connect:

```
$ qshare
```

- [Screenshots](#screenshots)
- [Highlights](#highlights)
- [Features](#features)
- [Install](#install)
- [Usage](#usage)
- [Web UI](#web-ui)
- [Architecture](#architecture)
- [API](#api)
- [Development](#development)
- [Security](#security)
- [Contributing](#contributing)

---

## Screenshots

| Native GUI — live dashboard | Web — directory browse |
| --- | --- |
| ![Native GUI dashboard](docs/screenshots/gui-main.png) | ![Web list view](docs/screenshots/web-browse.png) |

| Web — grid view | Web — media preview |
| --- | --- |
| ![Web grid view](docs/screenshots/web-grid.png) | ![Web media preview](docs/screenshots/web-preview.png) |

| Web — search | Web — mobile |
| --- | --- |
| ![Web search](docs/screenshots/web-search.png) | ![Web mobile layout](docs/screenshots/web-mobile.png) |

---

## Highlights

- **Zero config** — pick a directory, get a URL. No accounts, no setup, no cloud.
- **Three front-ends** — SolidJS web UI, iced native GUI (macOS / Windows), ratatui TUI (Linux).
- **Live** — SSE pushes FS events to the browser; the GUI shows live connection stats.
- **Discoverable** — `_qshare._tcp.local.` mDNS service appears on the LAN automatically.
- **Fast** — directory TTL cache + Range downloads + image / video / PDF / code previews.
- **Tiny** — ~4 MB CLI binary, ~12 MB GUI. No Node needed at runtime (frontend is embedded).
- **Safe** — strict sandbox refuses `..` and absolute paths; read-only access.

---

## Features

- **Browse** — virtual-scrolled list or thumbnail grid, sort by name / size / mtime.
- **Search** — recursive substring / regex over filenames, with a dedicated results page.
- **Preview** — images, video (HTTP Range, seeks), PDF, syntax-highlighted code, plain text.
- **Download** — single-click, supports pause / resume / seek via `Range`.
- **Live updates** — file changes appear without a refresh.
- **Mobile** — responsive layout, touch gestures for back / up.
- **Favorites & pinning** — sidebar bookmarks and a pinned-directory toolbar button.

---

## Install

Pre-built binaries for Linux, macOS and Windows are attached to each [Release](https://github.com/qinwenhui/q-share/releases). You can also build from source:

```bash
git clone https://github.com/qinwenhui/q-share.git
cd q-share

# One-time: install JS deps for the frontend
just web-install   # or:  cd web && npm install

# Release build (embeds the frontend)
just build         # or:  cd web && npm run build && cd .. && cargo build --release
```

**Requires**: Rust 1.88+, Node 20+.

Build outputs:

| Binary | Crate | Description |
|---|---|---|
| `target/release/qshare` | `qshare-gui` | Native GUI (macOS / Windows) |
| `target/release/qshare-cli` | `qshare-cli` | Headless CLI |
| `target/release/qshare-tui` | `qshare-tui` | Terminal UI |

---

## Usage

### CLI — `qshare-cli`

```bash
qshare-cli --root <DIR> [options]

Options:
  -r, --root <DIR>        Directory to share
  -p, --port <PORT>       Port to listen on             [default: 8888]
      --host <IP>         Host/IP to bind               [default: 0.0.0.0]
      --show-hidden       Include dotfiles in listings
      --cache-ttl <SECS>  Directory cache TTL (seconds) [default: 5]
      --no-qr             Don't print QR code to stdout
```

Example:

```bash
qshare-cli --root ~/Movies --port 9000 --show-hidden
```

### GUI — `qshare`

```bash
qshare                    # zero-config — pick a folder in the UI
qshare --root ~/Movies    # preset root
qshare --port 9000        # override the default port (8888)
```

> The GUI always binds `0.0.0.0` so both loopback and LAN clients connect;
> the displayed URL uses the auto-detected LAN IP.

Features:

- Live QR + URL after **Start** (click to copy)
- Real-time **active** / **bytes** / **errors** counters
- Toast notifications on copy / start / stop
- Status pill in the header reflects server state (idle / starting / running / error)
- Switchable visual styles (hacker / tech / retro / anime)

### TUI — `qshare-tui`

```bash
qshare-tui ~/Movies              # positional root (default: current dir)
qshare-tui --root ~/Movies       # same, via flag (overrides positional)
qshare-tui ~/Movies -p 9000      # override port
```

Keyboard:

- **Command mode** (default) — `s` start · `x` stop · `q` quit · `Tab` edit settings
- **Edit mode** — type into the focused field · `↑`/`↓` (or `Tab` / `Shift+Tab`) move · `Enter` / `Esc` back to command mode
- `Ctrl+C` always quits

> In edit mode, letters are typed as-is — `q`/`s`/`x` are not commands, so
> paths containing them can be entered safely.

---

## Web UI

The browser is the primary surface for end-users. Open the printed URL on any device:

- **Browse** — virtual-scrolled list, sort by name / size / mtime, ascending / descending
- **Search** — recursive substring / regex over filenames
- **Preview** — images, video (HTTP Range), PDF, code (syntax-highlighted), plain text
- **Download** — single-click, supports pause / resume / seek
- **Live updates** — file changes appear without a refresh
- **Mobile** — responsive layout, touch gestures for back / up

The frontend is a SolidJS SPA (~48 KB JS) embedded into the binary at build time via `rust-embed`.

---

## Architecture

```
q-share/
├── crates/
│   ├── qshare-core     # axum server, FS sandbox, dir cache, thumbnails, mDNS, stats
│   ├── qshare-cli      # headless entry (clap + tracing)
│   ├── qshare-gui      # native iced app (macOS / Windows)
│   ├── qshare-tui      # ratatui dashboard (Linux / SSH)
│   └── qshare-assets   # rust-embed of web/dist/
├── web/                # SolidJS + Vite + TypeScript SPA
└── justfile            # build / dev / test recipes
```

### Request flow

```
GET /api/list?path=/photos
  └─► middleware::track (stats)
        └─► sandbox::resolve (no escape)
              └─► dir_cache.get(path) (TTL 5 s, invalidated by FS watcher)
                    └─► read_dir (parallel, sorted, paginated)
                          └─► JSON response
GET /api/raw?path=/photos/img.jpg (Range: bytes=0-1023)
  └─► middleware::track
        └─► sandbox::resolve
              └─► range::parse → tokio::fs::File → 206 Partial Content
GET /api/events
  └─► SSE stream: fs-change events (200 ms debounce) + 30 s heartbeat
GET /api/thumb?path=/photos/img.jpg
  └─► thumb cache (SHA-256 keyed on ~/.cache/q-share/thumbs/) → on miss → resize → cache
```

### Components

| Layer | Crate / module | Notes |
|---|---|---|
| HTTP server | `axum 0.8` | graceful shutdown, mDNS + watcher live for server lifetime |
| Sandbox | `qshare_core::fs::sandbox` | canonicalize + reject `..` + bound check |
| Directory cache | `qshare_core::cache` | TTL 5 s, invalidated by notify watcher |
| FS watcher | `notify 8` + debouncer | 200 ms window, broadcast to SSE clients |
| mDNS | `mdns-sd 0.11` | `_qshare._tcp.local.` with TXT `path=<root-label>` |
| QR | `qrcode 0.14` | SVG (web + GUI) + unicode half-blocks (TUI + CLI) |
| Thumbnails | `image 0.25` + `sha2` | disk cache keyed on path + size + mtime |
| Stats | `AtomicI64` + RAII guard | active, bytes, errors (exposed via `/api/stats`) |
| Embed | `rust-embed 8` | `web/dist/` compiled into `qshare-assets` |

---

## API

All endpoints under `/api/`:

| Endpoint | Purpose |
|---|---|
| `GET /api/list?path=&offset=&limit=&sort=` | Paginated directory listing |
| `GET /api/stat?path=` | Single file/dir metadata |
| `GET /api/raw?path=` | File body (supports `Range`) |
| `GET /api/thumb?path=&size=` | Cached thumbnail (JPEG) |
| `GET /api/search?q=&regex=&limit=` | Recursive search |
| `GET /api/events` | SSE: `fs-change` events |
| `GET /api/whoami` | Client IP + user agent |
| `GET /api/health` | `{"ok":true}` |
| `GET /api/stats` | Live counters (active, bytes_served, errors) |

---

## Development

```bash
# Terminal 1: backend on :8888
just dev

# Terminal 2: frontend dev server on :5173 (proxies /api to :8888)
just web-dev
```

Open <http://localhost:5173>.

```bash
just test    # cargo test --workspace
just check   # fmt + clippy (deny warnings)
just clean   # nuke target + node_modules
```

---

## Security

q-share is a **LAN file-sharing tool**, not a hardened internet-facing service:

- The shared root is served **read-only**; the sandbox rejects `..` and absolute-path escapes.
- There is **no authentication** — anyone who can reach the port can read the shared directory.
- Do **not** expose the port to the public internet.

See [SECURITY.md](SECURITY.md) for the full trust model and how to report a vulnerability.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). One PR, one change; explain the *why*.

---

## License

[MIT](LICENSE) © 2026 [qinwh](https://qinwh.cn)
