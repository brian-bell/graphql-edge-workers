# Research: Rust GraphQL on Cloudflare Workers

## Project Overview

Comparative study of two independent GraphQL server implementations targeting Cloudflare Workers (Rust → WASM). Both workers serve a flight log schema and resolve data via HTTP calls to an upstream origin API.

## Structure

Cargo workspace with two independent crates — no shared code between them:

- `workers/gql-async-graphql/` — Uses the `async-graphql` crate
- `workers/gql-custom-parser/` — Hand-written lexer/parser/executor (partial GraphQL spec)

## Build & Deploy

- **Target:** `wasm32-unknown-unknown` via `workers-rs`
- **Build chain:** `wrangler` → `worker-build` → `cargo build`
- **Local dev:** `wrangler dev` from within each worker directory
- **Deploy:** `wrangler deploy` (CI via GitHub Actions, one workflow per worker)
- **Rust version:** Pinned in `rust-toolchain.toml`

## Conventions

- Each worker is fully independent — do not introduce shared crates or cross-dependencies
- Resolvers only make HTTP calls (via `worker::Fetch`) to the origin API
- GraphQL endpoint: `POST /graphql`. Health check: `GET /health`
- Errors follow GraphQL spec: `{ "data": null, "errors": [...] }`, HTTP 200 always. Worker-level failures return HTTP 502.
- Origin base URL configured via Workers environment variable in `wrangler.toml`
- No middleware in v1 (auth, caching, rate limiting are planned for v2)

## Custom Parser Scope (gql-custom-parser)

Partial GraphQL spec compliance:
- Supported: fields, arguments, variables, aliases
- Not supported: fragments, directives, introspection, subscriptions

## Key Decisions

See `docs/adrs/` for Architecture Decision Records.
