# ADR 002: Rust + WASM via workers-rs

**Date:** 2026-03-08
**Status:** Accepted

## Context

Cloudflare Workers support multiple languages. For Rust, there are two paths:

1. **`workers-rs`** — Compiles to `wasm32-unknown-unknown`, uses `wasm-bindgen` for JS interop, gives full access to Workers APIs (KV, Fetch, environment variables).
2. **Generic WASI (`wasm32-wasip1`)** — Cloudflare has experimental WASI support via `workers-wasi`, but it wraps the binary in a JS shim and has limited syscall support.

## Decision

Use `workers-rs` targeting `wasm32-unknown-unknown`.

## Rationale

- WASI support on Cloudflare is still experimental with limited syscalls
- `workers-rs` provides direct access to platform APIs we'll need (Fetch for HTTP calls, environment variables for config)
- The portability goal is achieved through code architecture (platform-agnostic core logic) rather than compilation target
- `workers-rs` is the officially supported and documented path for Rust on Cloudflare

## Consequences

- Rust code must compile to `wasm32-unknown-unknown` — some crates that rely on OS-level features won't work
- Platform-specific code (entry point, Fetch API, env vars) lives in a thin layer, keeping the core portable
- Pinned via `rust-toolchain.toml` for reproducible builds
