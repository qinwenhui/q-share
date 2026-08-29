# Changelog

All notable changes to q-share are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/), and
this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-08-30

### Fixed

- GUI no longer defaults the share root to `/`. macOS launches GUI apps from
  Finder with cwd = `/`, so the old `current_dir()` default would share the
  entire filesystem on the first click. Now defaults to the user's home
  directory (`$HOME` / `%USERPROFILE%`).
- Release packaging is now per-platform: each archive contains every binary
  for that platform (Windows: GUI + CLI + TUI; macOS: `.app` + CLI + TUI;
  Linux: CLI + TUI). Previously all binaries for a platform wrote the same
  `q-share-<target>` archive name and overwrote each other, so a release
  ended up with a single binary per platform.

### Added

- App icons: macOS `.app` bundle icon (AppIcon.icns + Info.plist + ad-hoc
  signature) and a Windows icon embedded into `qshare.exe`; full 16–1024 px
  PNG / .icns / .ico asset set.

## [0.1.0] - 2026-08-29

Initial release.

### Added

- Zero-config LAN file sharing: pick a directory, get a URL + QR code.
- Three front-ends: SolidJS web UI, iced native GUI (macOS / Windows), ratatui TUI.
- Directory browsing with virtualized lists, sorting, and recursive search.
- Previews: images, video (HTTP Range), PDF, and syntax-highlighted code.
- Sandboxed read-only serving with `..`-escape protection.
- Live updates over WebSocket / SSE; file watcher with a 200 ms debounce.
- mDNS discovery (`_qshare._tcp.local.`).
- Directory TTL cache, disk-cached thumbnails, connection stats.
