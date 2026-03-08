# ADR 007: Build & Deploy Toolchain

**Date:** 2026-03-08
**Status:** Accepted

## Context

Rust + WASM targeting Cloudflare Workers requires a specific toolchain. We need reproducible builds locally and in CI.

## Decision

Use the standard Cloudflare Workers Rust toolchain:

| Tool | Purpose |
|---|---|
| rustup | Installs Rust + `wasm32-unknown-unknown` target |
| cargo | Build tool, manages workspace and dependencies |
| wrangler | Cloudflare CLI — builds, local dev, deploys |
| worker-build | Compiles Rust → WASM, generates JS glue (invoked by wrangler) |

Pin Rust version via `rust-toolchain.toml` at workspace root.

## CI/CD

Two independent GitHub Actions workflows (one per worker):

1. Trigger: push to `main` with changes in that worker's directory, or manual dispatch
2. Build: install Rust + wasm target, `cargo test`, `cargo build --release`
3. Deploy: `wrangler deploy` with Cloudflare API token

Required GitHub secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`.

## Rationale

- No external build system needed — wrangler orchestrates the full chain
- `rust-toolchain.toml` ensures CI and local builds use the same compiler
- Path-filtered workflows avoid deploying one worker when only the other changed
- Manual dispatch enables ad-hoc deploys

## Consequences

- Developers need rustup, wrangler, and Node.js (wrangler dependency) installed locally
- CI must install the wasm target explicitly (`rustup target add wasm32-unknown-unknown`)
- Each worker deploys independently — no coordinated releases needed
