# Edge GraphQL Server Design

**Date:** 2026-03-08
**Status:** Approved

## Goal

Build two independent GraphQL server implementations in Rust targeting Cloudflare Workers, to compare the async-graphql library approach against a hand-written parser.

## Platform

Cloudflare Workers via `workers-rs`, compiling Rust to `wasm32-unknown-unknown`. Architecture keeps core logic portable so a future WASI or Fastly adapter is feasible without rewriting business logic.

## Architecture

```
Client
  │
  ▼
Cloudflare Worker (edge)
  │
  ├─ 1. Parse incoming HTTP request (POST /graphql)
  ├─ 2. Extract GraphQL query/variables from JSON body
  ├─ 3. Parse & validate GraphQL operation
  ├─ 4. Execute resolvers (HTTP calls to origin)
  ├─ 5. Assemble JSON response
  └─ 6. Return HTTP response
  │
  ▼
Origin API (DigitalOcean)
  │
  ▼
Postgres
```

### Endpoints

- `POST /graphql` — GraphQL queries and mutations
- `GET /health` — Liveness check

### Error Handling

- GraphQL errors: `{ "data": null, "errors": [...] }`, HTTP 200
- Worker-level failures (origin unreachable): HTTP 502

### Configuration

- `ORIGIN_BASE_URL` — Workers environment variable set in `wrangler.toml`

## Schema

Flight log domain:

```graphql
type Query {
  flight(id: ID!): Flight
  flights(limit: Int, offset: Int): [Flight!]!
}

type Mutation {
  createFlight(input: CreateFlightInput!): Flight!
}

input CreateFlightInput {
  date: String!
  aircraftTitle: String
  aircraftRegistration: String
  departureIcao: String
  departureName: String
  departureLat: Float
  departureLon: Float
  arrivalIcao: String
  arrivalName: String
  arrivalLat: Float
  arrivalLon: Float
  distanceNm: Float
  elapsedSeconds: Int
  maxAltitudeFt: Float
  landingVsFpm: Float
  landingGForce: Float
  notes: String
}

type Flight {
  id: ID!
  date: String!
  aircraftTitle: String
  aircraftRegistration: String
  departureIcao: String
  departureName: String
  departureLat: Float
  departureLon: Float
  arrivalIcao: String
  arrivalName: String
  arrivalLat: Float
  arrivalLon: Float
  distanceNm: Float
  elapsedSeconds: Int
  maxAltitudeFt: Float
  landingVsFpm: Float
  landingGForce: Float
  notes: String
}
```

### Upstream API Contract

| Worker operation | Upstream call |
|---|---|
| `query { flight(id: "1") }` | `GET /flights/1` |
| `query { flights(limit: 10) }` | `GET /flights?limit=10&offset=0` |
| `mutation { createFlight(...) }` | `POST /flights` |

## Project Structure

```
research-rust-gql/
├── CLAUDE.md
├── Cargo.toml                  # workspace root
├── rust-toolchain.toml
├── workers/
│   ├── gql-async-graphql/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── schema.rs
│   │   │   └── http_client.rs
│   │   └── wrangler.toml
│   │
│   └── gql-custom-parser/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── parser/
│       │   │   ├── mod.rs
│       │   │   ├── lexer.rs
│       │   │   ├── ast.rs
│       │   │   └── validate.rs
│       │   ├── schema.rs
│       │   └── http_client.rs
│       └── wrangler.toml
├── .github/
│   └── workflows/
│       ├── gql-async-graphql.yml
│       └── gql-custom-parser.yml
├── docs/
│   ├── adrs/
│   └── plans/
└── README.md
```

## Two Implementations

### Project A: async-graphql

Uses the `async-graphql` crate. Schema defined via procedural macros (`#[Object]`, `#[derive(SimpleObject)]`). The library handles parsing, validation, execution, and response serialization.

### Project C: Custom Parser

Hand-written lexer, parser, and executor. Partial GraphQL spec:

- **Supported:** fields, arguments, variables, aliases
- **Not supported:** fragments, directives, introspection, subscriptions

Components: lexer → parser → AST → validator → executor.

## Build & Deploy

### Toolchain

| Tool | Purpose |
|---|---|
| rustup | Rust toolchain manager, installs `wasm32-unknown-unknown` target |
| cargo | Build tool / package manager, manages workspace |
| wrangler | Cloudflare CLI, builds Worker, local dev server, deploys |
| worker-build | Compiles Rust → WASM, generates JS glue |

### CI/CD

Two independent GitHub Actions workflows, one per worker:

1. Trigger on push to `main` with changes in that worker's directory, or manual dispatch
2. Install Rust + wasm target, `cargo test`, `cargo build --release`
3. `wrangler deploy` using Cloudflare API token

Required GitHub secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`

## Future Middleware (v2)

Not built in v1. Architecture must leave a clean seam for composition.

| Middleware | Purpose | Notes |
|---|---|---|
| Authentication | Validate JWT/API key | Before GraphQL parsing. Rejects with 401. |
| Authorization | Field/operation-level permissions | After parsing, before execution. |
| Rate limiting | Throttle by IP/key/user | Before parsing. Workers KV or Durable Objects for counters. |
| Caching | Cache upstream or full GraphQL responses | Query-level and/or resolver-level. Workers Cache API or KV. |
| Request logging | Structured observability | Operation name, duration, resolver timings, errors. |
| CORS | Cross-origin headers | Preflight handling on all responses. |
| Request validation | Size limits, depth/complexity limits | Before parsing. Abuse protection. |

Both workers should structure request handling as a pipeline with a clear "handle request" function separate from the Workers entry point, so middleware is just composition.

## Comparison Metrics

- Binary size (compiled `.wasm`)
- Cold start time
- Request latency (p50/p99)
- Developer ergonomics (subjective)
- Lines of code
- Compile time
