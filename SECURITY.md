# Security Policy

## Reporting a vulnerability

Please report security issues **privately** via GitHub's Private Vulnerability
Reporting (Repository → Security → Report a vulnerability) rather than opening
a public issue. We aim to respond within a few days.

## Scope and trust model

q-share is a **LAN file-sharing tool**, not a hardened internet-facing service.

- The shared root is served **read-only**. The sandbox (`qshare_core::fs::sandbox`)
  rejects `..` traversal and absolute-path escapes by canonicalizing and
  bounds-checking every path.
- There is **no authentication or authorization**. Any client that can reach
  the bound address can read the shared directory. The intended deployment is
  a trusted local network — do not expose the port to the public internet.
- The server binds `0.0.0.0` by default so all interfaces (Wi-Fi, direct
  Ethernet) work out of the box. The URL shown to the operator is what they
  share.

Out of scope: attacks requiring physical access, or a deliberately shared
directory (the operator chooses what to share).
