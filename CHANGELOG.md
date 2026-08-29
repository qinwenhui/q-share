# Changelog

All notable changes to q-share are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/), and
this project adheres to [Semantic Versioning](https://semver.org/).

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
