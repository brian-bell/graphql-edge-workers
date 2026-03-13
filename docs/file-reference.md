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
| `AGENTS.md` | Agent-agnostic instructions for AI coding agents. Canonical source for repo structure, build commands, architecture constraints, and code conventions. Read by Codex natively and Claude Code via reference from `CLAUDE.md`. |
| `CLAUDE.md` | Thin shim for Claude Code. Points to `AGENTS.md` for shared instructions and adds Claude-specific environment notes (e.g. `cargo` PATH). |

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
| `wrangler.toml` | Cloudflare Workers config. Sets compatibility date, entry point (`build/worker/shim.mjs`), build command (`worker-build --release`), and the `SUPABASE_URL` / `SUPABASE_PUBLISHABLE_KEY` environment variables. |
| `README.md` | Worker dev guide. Documents prerequisites (including Supabase project setup), local development, authenticated `curl` examples, service-to-service auth pattern, and deploy instructions. |

### Source Code

| File | Purpose |
|---|---|
| `src/lib.rs` | Worker entry point. The `#[event(fetch)]` handler routes incoming requests: `GET /health` to the health handler, `POST /graphql` to the GraphQL handler, everything else to a 404. Bridges between Cloudflare's `worker::Request`/`Response` types and standard `http::Request`/`Response`. |
| `src/handler.rs` | HTTP request handlers. `health()` returns `{"status":"ok"}`. `graphql()` lazily initializes the schema (via `OnceLock`), builds `RuntimeConfig` from the environment, runs JWT auth middleware (returning 401 on failure), enforces an 8 KB body size limit, parses the body as a GraphQL request, executes it, and returns the JSON response. Returns 502 on upstream errors. Includes tests for body size validation. |
| `src/schema.rs` | GraphQL schema definition. `QueryRoot` exposes `flight(id)` and `flights(limit, offset)` queries with input clamping (limit 0-100, offset >= 0). `MutationRoot` exposes `createFlight`, `updateFlight`, and `deleteFlight`. `build_schema()` constructs the schema with an injected `FlightApi` implementation. Contains 40+ unit tests using mock API implementations covering introspection, resolver behavior, pagination, and error handling. |
| `src/models.rs` | Data types. `Flight` is a `SimpleObject` struct with an `ID` primary key, required `date`, `user_id`, and optional fields for aircraft details, departure/arrival info, flight metrics, and notes. `FlightRow` is the PostgREST row shape (with `user_id`). `CreateFlightPayload` and `UpdateFlightPatch` are the REST body types for mutations. `CreateFlightInput` and `UpdateFlightInput` are `InputObject` structs for GraphQL mutations. All types derive `Serialize`/`Deserialize` with `rename_all = "camelCase"`. |
| `src/http_client.rs` | Supabase PostgREST client. `FlightApi` trait defines the contract (`get_flight`, `get_flights`, `create_flight`, `update_flight`, `delete_flight`) using boxed pinned futures (required because WASM futures are `!Send`). `SupabaseClient` implements the trait using Cloudflare's `Fetch` API, building PostgREST query-param URLs and attaching `apikey`, `Authorization` (bearer token), and `Prefer` headers. `SupabaseError` distinguishes HTTP status errors (with an `is_not_found()` helper) from network/parse errors. Call sites in `schema.rs` wrap futures in `SendWrapper` to satisfy async-graphql's `Send` bounds (safe because WASM is single-threaded). |
| `src/auth.rs` | JWT authentication. Parses the `Authorization: Bearer` header, fetches the Supabase JWKS endpoint with a TTL cache, verifies the token signature using WebCrypto (RS256/ES256), and validates standard claims (exp, aud, iss). Produces an `AuthContext` containing the authenticated user's `sub` (user ID). |
| `src/config.rs` | Runtime configuration. `RuntimeConfig` reads `SUPABASE_URL` and `SUPABASE_PUBLISHABLE_KEY` from the Worker environment and derives the PostgREST and JWKS URLs. `ConfigError` for missing/invalid vars. |

### Supabase

| File | Purpose |
|---|---|
| `supabase/rls.sql` | Row-Level Security policies for the `flights` table. Enforces per-user isolation so each user can only read, insert, update, and delete their own rows (matched by `user_id = auth.uid()`). |
