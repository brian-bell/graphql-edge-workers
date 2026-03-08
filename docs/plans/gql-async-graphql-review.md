# PR #1 Review: gql-async-graphql worker

Review of the initial implementation prior to merge into `main`.

---

## Resolved

### ID scalar for query/mutation input parameters

The `flight`, `updateFlight`, and `deleteFlight` resolver parameters used `String`
instead of `async_graphql::ID`, producing `flight(id: String!)` in the generated
schema instead of `flight(id: ID!)`. Fixed — the `FlightApi` trait keeps `String`
(HTTP client concern), and the resolvers convert via `.to_string()`.

---

## Suggestions for Future Work

### Content-Type validation on POST /graphql

The handler parses the body as JSON unconditionally. Per the GraphQL over HTTP spec,
it should validate `Content-Type: application/json` and return a clear error for
other media types. Currently, non-JSON content types produce confusing serde parse
errors.

### 404 fallback lacks Content-Type header

The health and graphql endpoints set `Content-Type: application/json`, but the 404
fallback returns bare text without a `Content-Type` header. Add
`Content-Type: text/plain` or return a JSON error body to match the other endpoints.

### Path injection hardening

The `id` parameter is interpolated directly into URL paths (`/flights/{id}`) in
`http_client.rs`. While `Url::parse` normalizes paths, URL-encoding or validating
the `id` format would prevent edge cases like `id: "../../admin"`.

### Query depth/complexity limiting

async-graphql supports `.limit_depth()` and `.limit_complexity()` on the schema
builder. The current schema is flat enough that abuse is unlikely, but these guards
are worth adding before production use.

### Implement `std::error::Error` for `OriginError`

`OriginError` implements `fmt::Display` but not `std::error::Error`. Adding the
`Error` impl is idiomatic Rust and enables `?` operator usage and integration with
error-chain crates.

### Pin `rust-toolchain.toml` for production

Currently uses `channel = "stable"` instead of a pinned version (plan specified
`1.83`). Fine for evaluation, but should be pinned to prevent unexpected breakage
from toolchain updates.

### `http` crate version divergence risk

`http = "1"` is listed as a direct dependency, but `worker` re-exports the `http`
crate. If `worker` upgrades its `http` dependency, the two versions could diverge.
Consider removing the direct dep or pinning it to match `worker`.

---

## Strengths

### FlightApi trait for testability

Not in the original plan — extracts a trait from `OriginClient`, enabling a
comprehensive mock-based test suite (22 tests) without any HTTP calls. This is the
single most impactful positive deviation from the plan.

### Dual-layer body size defense

Content-Length fast-reject plus `http_body_util::Limited` during collection handles
both honest and dishonest Content-Length headers. Well-tested with 5 handler tests.

### Schema caching via OnceLock

Avoids repeated env var reads and schema construction per request. Comments correctly
explain why this is race-free in single-threaded WASM.

### Typed OriginError enum

Replaces fragile string-based 404 detection (`e.contains("404")`) with a structured
`OriginError::Status(u16)` enum and `is_not_found()` method.

### SendWrapper usage

Correct and well-documented approach for bridging `!Send` WASM futures to
async-graphql's `Send + Sync` requirements.

### Opaque error messages

The 502 response for missing config says "Service misconfigured" — does not leak
env var names or internal details.

### Plan deviation documentation

All deviations are documented with rationale in
`docs/plans/gql-async-graphql-deviations.md`. Worth continuing this practice.
