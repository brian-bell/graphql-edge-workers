# Agent Instructions

Rust GraphQL flight log API on Cloudflare Workers (WASM), backed by Supabase (PostgREST + Auth).

## Repo Structure

```
Cargo.toml                      # workspace root
rust-toolchain.toml             # pinned Rust version + wasm target
workers/
  gql-async-graphql/            # implementation using async-graphql crate
  gql-custom-parser/            # implementation using hand-written parser
docs/
  adrs/                         # architecture decision records
  plans/                        # implementation plans + task persistence
.github/workflows/              # one CI/CD workflow per worker
```

## Build & Test Commands

**Run tests** (native, not WASM):
```sh
cargo test -p gql-async-graphql
```

**Build for WASM**:
```sh
cargo build --target wasm32-unknown-unknown --release -p gql-async-graphql
```

**Run locally** (from worker directory):
```sh
cd workers/gql-async-graphql && npx wrangler dev
```

## Architecture Constraints

- Each worker crate is fully independent — no shared crates or cross-dependencies
- Resolvers call Supabase PostgREST over HTTP — no direct database access
- All requests require a valid Supabase JWT; the worker verifies tokens against the project JWKS
- The verified JWT is forwarded to Supabase so Row-Level Security (RLS) enforces per-user data isolation
- Environment variables: `SUPABASE_URL`, `SUPABASE_PUBLISHABLE_KEY` (set in `wrangler.toml`)

## Code Conventions

- Stable Rust toolchain, `wasm32-unknown-unknown` target
- Worker crates use `cdylib` + `rlib` library targets
- WASM futures are `!Send`; use `SendWrapper` to satisfy async-graphql's `Send` bounds (safe because WASM is single-threaded)
- Request body size limited to 8 KB (checked via `Content-Length` header and `Limited` wrapper)
- `RuntimeConfig` derives the PostgREST and JWKS URLs from the base `SUPABASE_URL`

## Key Documentation

- `docs/adrs/` — architecture decision records
- `docs/plans/` — implementation plans
- `workers/gql-async-graphql/README.md` — worker-level dev guide (Supabase setup, curl examples, deploy)
