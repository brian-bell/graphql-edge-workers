# async-graphql: Plan vs Implementation

How the `gql-async-graphql` worker deviates from or builds upon the plans in
`2026-03-08-edge-graphql-design.md` and `2026-03-08-gql-async-graphql.md`.

---

## Additions Beyond the Plan

### Update and Delete Mutations

The design spec defined a single mutation:

```graphql
createFlight(input: CreateFlightInput!): Flight!
```

The implementation adds two more:

```graphql
updateFlight(id: ID!, input: UpdateFlightInput!): Flight!
deleteFlight(id: ID!): Boolean!
```

This required a new `UpdateFlightInput` type (all fields optional, unlike
`CreateFlightInput` where `date` is required) and two new HTTP client methods
(`PUT /flights/{id}`, `DELETE /flights/{id}`).

### Body Size Limiting

Not mentioned anywhere in the plan. The implementation enforces an 8 KB limit
with a two-layer defense:

1. **Fast-reject** — checks the `Content-Length` header before reading any bytes.
2. **Hard limit** — uses `http_body_util::Limited` during body collection to
   catch missing or dishonest `Content-Length` headers.

Oversized requests get a `413 Payload Too Large` response. This is a new HTTP
status code not in the plan (which only specified 200 and 502).

### Schema Caching via OnceLock

The plan's handler pseudocode reads `ORIGIN_BASE_URL` and builds the schema on
every request. The implementation caches the schema in a `static OnceLock`,
so the env var is read and the schema is constructed only once. A comment
explains why this is safe in single-threaded WASM.

### FlightApi Trait for Testability

The plan specified a concrete `OriginClient` with `get()` and `post()` methods.
The implementation extracts a `FlightApi` trait with pinned boxed futures, which
enables mock-based unit tests without HTTP. The resolvers accept
`Box<dyn FlightApi>` via async-graphql's context data, and the test suite
provides a `MockFlightApi` that returns canned responses.

### Pagination Clamping

The plan specified default values for `flights(limit, offset)` — limit defaults
to 20, offset defaults to 0. The implementation goes further:

- `limit` is clamped to `[0, 100]` (prevents unbounded queries)
- `offset` is clamped to `>= 0` (prevents negative offsets)

### SendWrapper for WASM Safety

The plan doesn't mention thread-safety concerns. In practice,
`worker::Fetch` returns `!Send` futures, but async-graphql requires
`Send + Sync` on context data. The implementation wraps origin API calls with
`send_wrapper::SendWrapper` at the call site in `schema.rs`, which is sound
because WASM is single-threaded.

---

## Structural Deviations

### HTTP Client Generalization

The plan specified two methods:

- `get(path) -> Result<T>`
- `post(path, body) -> Result<T>`

The implementation has three:

| Method | Purpose |
|---|---|
| `get<T>(path)` | GET requests with JSON deserialization |
| `send_json<T, B>(method, path, body)` | Generic JSON body requests (POST, PUT) |
| `delete(path)` | DELETE requests, returns `()` |

`send_json` is parameterized on HTTP method rather than having separate `post()`
and `put()` helpers — a consequence of adding `updateFlight`.

### Error Typing

The plan mentioned error handling in broad strokes (GraphQL errors at HTTP 200,
worker failures at HTTP 502). The implementation introduces a structured
`OriginError` enum:

```rust
enum OriginError {
    Status(u16),    // HTTP error from origin
    Other(String),  // network, parse, URL errors
}
```

The `is_not_found()` method on this enum is what lets the `flight()` resolver
convert 404s to `null` while propagating other errors.

### Flight ID Scalar Type

The plan's schema used `id: String` on the Flight type. The implementation uses
`async_graphql::ID`, which maps to the GraphQL `ID` scalar — semantically
correct for entity identifiers and conventional in GraphQL APIs.

---

## Planned Items Not Yet Implemented

### Comparison Metrics Collection

The design doc lists binary size, cold start latency, p50/p99 response times,
lines of code, and compile time as comparison targets. No instrumentation or
benchmarking harness exists yet.

### v2 Middleware

The design doc explicitly defers these to "v2, not in v1": authentication,
authorization, rate limiting, caching, request logging, CORS, and request
validation. None are implemented (body size limiting aside, which is a form of
request validation).

---

## Dependencies Not in the Original Plan

| Crate | Why |
|---|---|
| `http-body-util` | Body size limiting via `Limited` |
| `send_wrapper` | Wrapping `!Send` WASM futures for async-graphql |
| `wasm-bindgen-futures` | WASM async/await interop (implicit in plan, explicit in Cargo.toml) |
| `tokio` (dev) | Test runtime for async unit tests |

---

## Test Strategy Deviation

The plan described two testing levels:

1. **Schema compilation tests** — "the schema compiles" as a smoke test.
2. **Manual end-to-end smoke tests** — curl commands against a running worker.

The implementation has a substantially richer test suite:

- Schema introspection tests verifying field names, types, and nullability
- Mock-based resolver tests covering success paths, 404 handling, and error propagation
- Pagination boundary tests (default values, clamping)
- Handler-level tests for body size rejection
- ~60 total test assertions

The `FlightApi` trait (not in the plan) is what made this level of testing
possible without an HTTP server or origin API.
