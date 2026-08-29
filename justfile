# q-share build & dev recipes
# Install: cargo install just
# (or `brew install just` on macOS)

# Default: list recipes
default:
    @just --list

# ──── Frontend ────

# Install frontend dependencies
web-install:
    cd web && npm install

# Frontend dev server with API proxy (requires qshare running on :8888)
web-dev:
    cd web && npm run dev

# Build the frontend into web/dist/
web-build:
    cd web && npm run build

# ──── Backend ────

# Dev build of the CLI
dev:
    cargo build -p qshare-cli

# Release build of the CLI (embeds the frontend)
build: web-build
    cargo build --release -p qshare-cli
    @echo "✓ built target/release/qshare-cli"

# Run tests
test:
    cargo test --workspace

# Format + lint
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# Clean all build artifacts
clean:
    cargo clean
    rm -rf web/dist web/node_modules

# ──── Convenience ────

# Quick local run sharing the current directory
run dir="./" port="8888":
    cargo run -p qshare-cli -- --root {{dir}} --port {{port}}
