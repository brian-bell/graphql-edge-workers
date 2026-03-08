# Rust GraphQL on Cloudflare Workers

Comparative study of two GraphQL server implementations in Rust targeting Cloudflare Workers (WASM). Once evaluated, the chosen approach becomes the production server. Both serve a flight log schema and resolve data via HTTP calls to an upstream origin API.

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

## Key Constraints

- Each worker crate is fully independent — no shared crates or cross-dependencies
- Resolvers only make HTTP calls to the origin API, no direct DB access
- See `docs/adrs/` for all architectural decisions
- See `docs/plans/` for implementation plans
