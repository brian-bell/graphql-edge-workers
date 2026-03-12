# File Reference

Purpose of every file in the repository, excluding the `docs/` directory.

---

## Root

| File | Purpose |
|---|---|
| `Cargo.toml` | Workspace root. Lists `workers/gql-async-graphql` as the sole member. Configures release profile with `opt-level = "s"` and LTO for small WASM binaries. |
| `Cargo.lock` | Pinned dependency versions for reproducible builds. |
| `rust-toolchain.toml` | Pins the stable Rust channel and adds the `wasm32-unknown-unknown` target so all contributors compile with the same toolchain. |
| `.gitignore` | Excludes `target/`, `build/`, `.worktrees/`, and `.wrangler/`. |
| `CLAUDE.md` | Project-level instructions for AI assistants. Describes repo structure, key constraints (no shared crates, HTTP-only resolvers), and pointers to ADRs and plans. |

---

## `.github/workflows/`

| File | Purpose |
|---|---|
| `gql-async-graphql.yml` | CI/CD pipeline for the async-graphql worker. Triggers on pushes to `main` (scoped to relevant paths) and PRs. Installs Rust + WASM target, runs `cargo test`, builds `--target wasm32-unknown-unknown --release`. Deploy step exists but is disabled pending Cloudflare secrets. |

---

## `workers/gql-async-graphql/`

### Configuration

| File | Purpose |
|---|---|
| `Cargo.toml` | Crate manifest. Declares `cdylib` + `rlib` library targets. Key deps: `worker` (Cloudflare SDK), `async-graphql`, `serde`/`serde_json`, `http`, `send_wrapper`, `wasm-bindgen-futures`. Dev dep: `tokio` (for running async tests). |
| `wrangler.toml` | Cloudflare Workers config. Sets compatibility date, entry point (`build/worker/shim.mjs`), build command (`worker-build --release`), and the `ORIGIN_BASE_URL` environment variable. |
| `README.md` | Local development guide. Documents prerequisites, how to run locally (`npx wrangler dev`), example `curl` commands for the `/health` and `/graphql` endpoints, and deploy instructions. |

### Source Code

| File | Purpose |
|---|---|
| `src/lib.rs` | Worker entry point. The `#[event(fetch)]` handler routes incoming requests: `GET /health` to the health handler, `POST /graphql` to the GraphQL handler, everything else to a 404. Bridges between Cloudflare's `worker::Request`/`Response` types and standard `http::Request`/`Response`. |
| `src/handler.rs` | HTTP request handlers. `health()` returns `{"status":"ok"}`. `graphql()` lazily initializes the schema (via `OnceLock`), reads `ORIGIN_BASE_URL` from the environment, enforces an 8 KB body size limit (checked both via `Content-Length` header and a `Limited` wrapper during body reads), parses the body as a GraphQL request, executes it, and returns the JSON response. Includes tests for body size validation. |
| `src/schema.rs` | GraphQL schema definition. `QueryRoot` exposes `flight(id)` and `flights(limit, offset)` queries with input clamping (limit 0-100, offset >= 0). `MutationRoot` exposes `createFlight`, `updateFlight`, and `deleteFlight`. `build_schema()` constructs the schema with an injected `FlightApi` implementation. Contains 40+ unit tests using mock API implementations covering introspection, resolver behavior, pagination, and error handling. |
| `src/models.rs` | Data types. `Flight` is a `SimpleObject` struct with an `ID` primary key, required `date`, and optional fields for aircraft details, departure/arrival info, flight metrics, and notes. `CreateFlightInput` and `UpdateFlightInput` are `InputObject` structs for mutations (update has all fields optional). All types derive `Serialize`/`Deserialize` with `rename_all = "camelCase"`. |
| `src/http_client.rs` | Upstream API client. `FlightApi` trait defines the contract (`get_flight`, `get_flights`, `create_flight`, `update_flight`, `delete_flight`) using boxed pinned futures (required because WASM futures are `!Send`). `OriginClient` implements the trait using Cloudflare's `Fetch` API, mapping REST endpoints (`GET /flights/{id}`, `GET /flights?limit=&offset=`, `POST /flights`, `PUT /flights/{id}`, `DELETE /flights/{id}`). `OriginError` distinguishes HTTP status errors (with an `is_not_found()` helper) from network/parse errors. Call sites in `schema.rs` wrap futures in `SendWrapper` to satisfy async-graphql's `Send` bounds (safe because WASM is single-threaded). |
