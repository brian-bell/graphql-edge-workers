# Rust GraphQL on Cloudflare Workers

GraphQL flight log API built in Rust, deployed to Cloudflare Workers (WASM). Data lives in Supabase (Postgres + PostgREST); the worker authenticates requests via Supabase JWT and forwards tokens so Row-Level Security enforces per-user isolation.

## Repo Structure

```
Cargo.toml                      # workspace root
rust-toolchain.toml             # pinned Rust version + wasm target
workers/
  gql-async-graphql/            # implementation using async-graphql crate
  gql-custom-parser/            # implementation using hand-written parser
infra/
  terraform/
    cloudflare/                 # Cloudflare worker + Access infrastructure
docs/
  adrs/                         # architecture decision records
  plans/                        # implementation plans + task persistence
.github/workflows/              # one CI/CD workflow per worker
```

## Quick Start

### Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- Node.js (for the `wrangler` CLI)
- A Supabase project with a `flights` table exposed through PostgREST

### Run locally

Set `SUPABASE_URL` and `SUPABASE_PUBLISHABLE_KEY` in `workers/gql-async-graphql/wrangler.toml`, then:

```sh
cd workers/gql-async-graphql
npx wrangler dev
```

### Example requests

```sh
# Health check
curl http://localhost:8787/health

# Query flights (requires a Supabase access token)
curl -X POST http://localhost:8787/graphql \
  -H "Authorization: Bearer $SUPABASE_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights(limit: 5) { id date aircraftTitle } }"}'
```

## Run Tests

```sh
cargo test -p gql-async-graphql
```

## Architecture

- Each worker crate is independent — no shared crates or cross-dependencies
- Resolvers call Supabase PostgREST over HTTP; no direct database access
- JWT auth verified against the Supabase project JWKS; token forwarded for RLS
- Service-to-service auth uses dedicated Supabase user accounts — see [ADR 008](docs/adrs/008-service-to-service-auth.md)
- See `docs/adrs/` for all architecture decision records

## Documentation

| Path | Description |
|---|---|
| `docs/adrs/` | Architecture decision records |
| `docs/plans/` | Implementation plans |
| `docs/file-reference.md` | Purpose of every file in the repo |
| `workers/gql-async-graphql/README.md` | Worker dev guide — Supabase setup, endpoints, curl examples, deploy |
| `AGENTS.md` | Instructions for AI coding agents |
