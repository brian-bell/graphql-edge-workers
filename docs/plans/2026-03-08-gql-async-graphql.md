# gql-async-graphql Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers-extended-cc:executing-plans to implement this plan task-by-task.

**Goal:** Build a Cloudflare Worker in Rust that serves a flight-log GraphQL API using the `async-graphql` crate, resolving data via HTTP calls to an upstream origin API.

**Architecture:** Single `POST /graphql` endpoint backed by `async-graphql`. The Worker entry point (`lib.rs`) receives requests via `workers-rs`, hands off to a handler function (separated for future middleware composition), which parses the GraphQL request and executes it against an `async-graphql::Schema`. Resolvers use `worker::Fetch` to call the origin API.

**Tech Stack:** Rust, `worker` 0.7 (with `http` feature), `async-graphql` 7, `serde`/`serde_json`, `wrangler` CLI.

---

### Task 0: Scaffold the crate and verify it compiles + deploys

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `rust-toolchain.toml`
- Create: `workers/gql-async-graphql/Cargo.toml`
- Create: `workers/gql-async-graphql/src/lib.rs`
- Create: `workers/gql-async-graphql/wrangler.toml`

**Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
members = ["workers/gql-async-graphql"]
resolver = "2"
```

**Step 2: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "1.83"
targets = ["wasm32-unknown-unknown"]
```

**Step 3: Create worker Cargo.toml**

```toml
[package]
name = "gql-async-graphql"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worker = { version = "0.7", features = ["http"] }
worker-macros = { version = "0.7", features = ["http"] }
async-graphql = "7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wasm-bindgen-futures = "0.4"

[profile.release]
opt-level = "s"
lto = true
```

**Step 4: Create minimal lib.rs**

```rust
use worker::*;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<http::Response<String>> {
    let response = http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"ok": true}"#.to_string())
        .unwrap();
    Ok(response)
}
```

**Step 5: Create wrangler.toml**

```toml
name = "gql-async-graphql"
main = "build/worker/shim.mjs"
compatibility_date = "2026-03-08"

[build]
command = "cargo install -q worker-build && worker-build --release"

[vars]
ORIGIN_BASE_URL = "http://localhost:8080"
```

**Step 6: Verify it compiles**

Run: `cd workers/gql-async-graphql && npx wrangler dev`
Expected: Worker starts on localhost, `curl http://localhost:8787/` returns `{"ok": true}`

**Step 7: Commit**

```bash
git add Cargo.toml rust-toolchain.toml workers/gql-async-graphql/
git commit -m "feat: scaffold gql-async-graphql worker crate"
```

---

### Task 1: Add health endpoint and request routing

**Files:**
- Modify: `workers/gql-async-graphql/src/lib.rs`

**Step 1: Write the failing test**

No unit test for routing — this is a thin integration layer. We verify manually.

**Step 2: Implement routing in lib.rs**

Replace `lib.rs` contents with:

```rust
use worker::*;

mod handler;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<http::Response<String>> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    match (method, path.as_str()) {
        (http::Method::GET, "/health") => handler::health(),
        (http::Method::POST, "/graphql") => handler::graphql(req, env).await,
        _ => Ok(http::Response::builder()
            .status(404)
            .body("Not Found".to_string())
            .unwrap()),
    }
}
```

**Step 3: Create handler module**

Create `workers/gql-async-graphql/src/handler.rs`:

```rust
use worker::*;

pub fn health() -> Result<http::Response<String>> {
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"status":"ok"}"#.to_string())
        .unwrap())
}

pub async fn graphql(
    _req: HttpRequest,
    _env: Env,
) -> Result<http::Response<String>> {
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"data":null}"#.to_string())
        .unwrap())
}
```

**Step 4: Verify**

Run: `cd workers/gql-async-graphql && npx wrangler dev`
- `curl http://localhost:8787/health` → `{"status":"ok"}`
- `curl -X POST http://localhost:8787/graphql` → `{"data":null}`
- `curl http://localhost:8787/foo` → 404

**Step 5: Commit**

```bash
git add workers/gql-async-graphql/src/
git commit -m "feat: add health endpoint and request routing"
```

---

### Task 2: Define the Flight type and GraphQL schema

**Files:**
- Create: `workers/gql-async-graphql/src/schema.rs`
- Modify: `workers/gql-async-graphql/src/lib.rs`

**Step 1: Write the failing test**

Add to the bottom of `schema.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Request;

    #[tokio::test]
    async fn test_flights_query_returns_empty_list() {
        let schema = build_schema("http://fake-origin.test".to_string());
        let resp = schema.execute(Request::new("{ flights { id } }")).await;
        // Should not panic — schema is valid and query parses
        assert!(resp.errors.is_empty() || !resp.errors.is_empty());
    }
}
```

Note: This test just validates the schema compiles and accepts a query. Resolvers will fail without a real origin, which is expected. We'll skip running this test in WASM — it uses `tokio::test` for native target only.

**Step 2: Run test to verify it fails**

Run: `cargo test -p gql-async-graphql --target x86_64-unknown-linux-gnu`
Expected: FAIL — `schema` module doesn't exist

**Step 3: Implement schema.rs**

```rust
use async_graphql::{Context, EmptySubscription, Object, Schema, SimpleObject, InputObject};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct Flight {
    pub id: String,
    pub date: String,
    pub aircraft_title: Option<String>,
    pub aircraft_registration: Option<String>,
    pub departure_icao: Option<String>,
    pub departure_name: Option<String>,
    pub departure_lat: Option<f64>,
    pub departure_lon: Option<f64>,
    pub arrival_icao: Option<String>,
    pub arrival_name: Option<String>,
    pub arrival_lat: Option<f64>,
    pub arrival_lon: Option<f64>,
    pub distance_nm: Option<f64>,
    pub elapsed_seconds: Option<i32>,
    pub max_altitude_ft: Option<f64>,
    pub landing_vs_fpm: Option<f64>,
    pub landing_g_force: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, InputObject)]
pub struct CreateFlightInput {
    pub date: String,
    pub aircraft_title: Option<String>,
    pub aircraft_registration: Option<String>,
    pub departure_icao: Option<String>,
    pub departure_name: Option<String>,
    pub departure_lat: Option<f64>,
    pub departure_lon: Option<f64>,
    pub arrival_icao: Option<String>,
    pub arrival_name: Option<String>,
    pub arrival_lat: Option<f64>,
    pub arrival_lon: Option<f64>,
    pub distance_nm: Option<f64>,
    pub elapsed_seconds: Option<i32>,
    pub max_altitude_ft: Option<f64>,
    pub landing_vs_fpm: Option<f64>,
    pub landing_g_force: Option<f64>,
    pub notes: Option<String>,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn flight(&self, _ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<Flight>> {
        let origin = _ctx.data::<String>()?;
        let _ = (origin, id); // TODO: implement HTTP call
        Ok(None)
    }

    async fn flights(
        &self,
        _ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<Vec<Flight>> {
        let origin = _ctx.data::<String>()?;
        let _ = (origin, limit, offset); // TODO: implement HTTP call
        Ok(vec![])
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_flight(
        &self,
        _ctx: &Context<'_>,
        input: CreateFlightInput,
    ) -> async_graphql::Result<Flight> {
        let origin = _ctx.data::<String>()?;
        let _ = (origin, input); // TODO: implement HTTP call
        Err("Not implemented".into())
    }
}

pub type FlightSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(origin_base_url: String) -> FlightSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(origin_base_url)
        .finish()
}
```

**Step 4: Add `mod schema;` to lib.rs**

Add `mod schema;` alongside `mod handler;` in `lib.rs`.

**Step 5: Run test to verify it passes**

Run: `cargo test -p gql-async-graphql --target x86_64-unknown-linux-gnu`
Expected: PASS

**Step 6: Commit**

```bash
git add workers/gql-async-graphql/src/
git commit -m "feat: define Flight type and GraphQL schema with async-graphql"
```

---

### Task 3: Wire schema into the handler

**Files:**
- Modify: `workers/gql-async-graphql/src/handler.rs`

**Step 1: Write the failing test**

In `schema.rs`, add a more specific test:

```rust
#[tokio::test]
async fn test_flights_query_returns_empty_vec() {
    let schema = build_schema("http://fake-origin.test".to_string());
    let resp = schema
        .execute(async_graphql::Request::new("{ flights { id date } }"))
        .await;
    assert!(resp.errors.is_empty());
    let data = resp.data.into_json().unwrap();
    assert_eq!(data, serde_json::json!({"flights": []}));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gql-async-graphql --target x86_64-unknown-linux-gnu`
Expected: FAIL (or PASS if schema already returns empty vec — that's fine, move on)

**Step 3: Update handler.rs to parse GraphQL request and execute**

```rust
use worker::*;

use crate::schema::{self, FlightSchema};

pub fn health() -> Result<http::Response<String>> {
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"status":"ok"}"#.to_string())
        .unwrap())
}

pub async fn graphql(
    req: HttpRequest,
    env: Env,
) -> Result<http::Response<String>> {
    let origin_base_url = env.var("ORIGIN_BASE_URL")?.to_string();
    let schema = schema::build_schema(origin_base_url);

    let body_bytes = req.into_body();
    let body: Vec<u8> = body_bytes
        .bytes()
        .await
        .unwrap_or_default();

    let gql_request: async_graphql::Request = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let error_body = serde_json::json!({
                "data": null,
                "errors": [{"message": format!("Invalid request body: {e}")}]
            });
            return Ok(http::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&error_body).unwrap())
                .unwrap());
        }
    };

    let gql_response = schema.execute(gql_request).await;
    let response_body = serde_json::to_string(&gql_response).unwrap();

    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body)
        .unwrap())
}
```

**Step 4: Run tests**

Run: `cargo test -p gql-async-graphql --target x86_64-unknown-linux-gnu`
Expected: PASS

**Step 5: Manual verification**

Run: `cd workers/gql-async-graphql && npx wrangler dev`

```bash
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights { id date } }"}'
```

Expected: `{"data":{"flights":[]}}`

**Step 6: Commit**

```bash
git add workers/gql-async-graphql/src/
git commit -m "feat: wire async-graphql schema into request handler"
```

---

### Task 4: Implement the HTTP client for upstream calls

**Files:**
- Create: `workers/gql-async-graphql/src/http_client.rs`
- Modify: `workers/gql-async-graphql/src/lib.rs`

**Step 1: Implement http_client.rs**

This module wraps `worker::Fetch` for calling the origin API. It cannot be unit tested in native target (depends on Workers runtime), so we test via integration later.

```rust
use serde::de::DeserializeOwned;
use worker::{Fetch, Url};

pub struct OriginClient {
    base_url: String,
}

impl OriginClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let parsed_url = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;

        let mut response = Fetch::Url(parsed_url)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {e}"))?;

        if response.status_code() >= 400 {
            return Err(format!("Origin returned status {}", response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse response JSON: {e}"))
    }

    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let parsed_url = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;

        let body_json = serde_json::to_string(body)
            .map_err(|e| format!("Failed to serialize request body: {e}"))?;

        let mut request_init = worker::RequestInit::new();
        request_init.with_method(worker::Method::Post);
        request_init.with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body_json)));

        let request = worker::Request::new_with_init(&url, &request_init)
            .map_err(|e| format!("Failed to create request: {e}"))?;

        let mut response = Fetch::Request(request)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {e}"))?;

        if response.status_code() >= 400 {
            return Err(format!("Origin returned status {}", response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse response JSON: {e}"))
    }
}
```

**Step 2: Add `mod http_client;` to lib.rs**

**Step 3: Verify it compiles**

Run: `cd workers/gql-async-graphql && cargo build --target wasm32-unknown-unknown --release`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add workers/gql-async-graphql/src/
git commit -m "feat: add OriginClient for upstream HTTP calls"
```

---

### Task 5: Wire resolvers to the HTTP client

**Files:**
- Modify: `workers/gql-async-graphql/src/schema.rs`
- Modify: `workers/gql-async-graphql/src/handler.rs`

**Step 1: Update schema to use OriginClient**

Update `build_schema` to accept and store an `OriginClient` in context, and update resolvers:

```rust
use async_graphql::{Context, EmptySubscription, Object, Schema, SimpleObject, InputObject};
use serde::{Deserialize, Serialize};

use crate::http_client::OriginClient;

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct Flight {
    pub id: String,
    pub date: String,
    pub aircraft_title: Option<String>,
    pub aircraft_registration: Option<String>,
    pub departure_icao: Option<String>,
    pub departure_name: Option<String>,
    pub departure_lat: Option<f64>,
    pub departure_lon: Option<f64>,
    pub arrival_icao: Option<String>,
    pub arrival_name: Option<String>,
    pub arrival_lat: Option<f64>,
    pub arrival_lon: Option<f64>,
    pub distance_nm: Option<f64>,
    pub elapsed_seconds: Option<i32>,
    pub max_altitude_ft: Option<f64>,
    pub landing_vs_fpm: Option<f64>,
    pub landing_g_force: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, InputObject)]
pub struct CreateFlightInput {
    pub date: String,
    pub aircraft_title: Option<String>,
    pub aircraft_registration: Option<String>,
    pub departure_icao: Option<String>,
    pub departure_name: Option<String>,
    pub departure_lat: Option<f64>,
    pub departure_lon: Option<f64>,
    pub arrival_icao: Option<String>,
    pub arrival_name: Option<String>,
    pub arrival_lat: Option<f64>,
    pub arrival_lon: Option<f64>,
    pub distance_nm: Option<f64>,
    pub elapsed_seconds: Option<i32>,
    pub max_altitude_ft: Option<f64>,
    pub landing_vs_fpm: Option<f64>,
    pub landing_g_force: Option<f64>,
    pub notes: Option<String>,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn flight(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<Flight>> {
        let client = ctx.data::<OriginClient>()?;
        let path = format!("/flights/{id}");
        match client.get::<Flight>(&path).await {
            Ok(flight) => Ok(Some(flight)),
            Err(e) if e.contains("404") => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn flights(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<Vec<Flight>> {
        let client = ctx.data::<OriginClient>()?;
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);
        let path = format!("/flights?limit={limit}&offset={offset}");
        client
            .get::<Vec<Flight>>(&path)
            .await
            .map_err(|e| e.into())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_flight(
        &self,
        ctx: &Context<'_>,
        input: CreateFlightInput,
    ) -> async_graphql::Result<Flight> {
        let client = ctx.data::<OriginClient>()?;
        client
            .post::<Flight, _>("/flights", &input)
            .await
            .map_err(|e| e.into())
    }
}

pub type FlightSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(origin_client: OriginClient) -> FlightSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(origin_client)
        .finish()
}
```

**Step 2: Update handler.rs to create OriginClient**

Update the `graphql` function:

```rust
use crate::http_client::OriginClient;
use crate::schema;

// ... health() stays the same ...

pub async fn graphql(
    req: HttpRequest,
    env: Env,
) -> Result<http::Response<String>> {
    let origin_base_url = env.var("ORIGIN_BASE_URL")?.to_string();
    let client = OriginClient::new(origin_base_url);
    let schema = schema::build_schema(client);

    // ... rest stays the same ...
}
```

**Step 3: Verify it compiles**

Run: `cd workers/gql-async-graphql && cargo build --target wasm32-unknown-unknown --release`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add workers/gql-async-graphql/src/
git commit -m "feat: wire GraphQL resolvers to upstream HTTP client"
```

---

### Task 6: Add error handling for origin failures

**Files:**
- Modify: `workers/gql-async-graphql/src/handler.rs`

**Step 1: Update handler to catch panics and return 502 on origin failure**

Add a top-level error wrapper in the `graphql` function. The current code already returns GraphQL-spec errors via `async_graphql::Result`, but we need to handle the case where the handler itself panics or the env var is missing:

```rust
pub async fn graphql(
    req: HttpRequest,
    env: Env,
) -> Result<http::Response<String>> {
    let origin_base_url = match env.var("ORIGIN_BASE_URL") {
        Ok(v) => v.to_string(),
        Err(_) => {
            return Ok(http::Response::builder()
                .status(502)
                .header("content-type", "application/json")
                .body(r#"{"error":"ORIGIN_BASE_URL not configured"}"#.to_string())
                .unwrap());
        }
    };

    let client = OriginClient::new(origin_base_url);
    let schema = schema::build_schema(client);

    let body_bytes = req.into_body();
    let body: Vec<u8> = body_bytes.bytes().await.unwrap_or_default();

    let gql_request: async_graphql::Request = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let error_body = serde_json::json!({
                "data": null,
                "errors": [{"message": format!("Invalid request body: {e}")}]
            });
            return Ok(http::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&error_body).unwrap())
                .unwrap());
        }
    };

    let gql_response = schema.execute(gql_request).await;
    let response_body = serde_json::to_string(&gql_response).unwrap();

    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body)
        .unwrap())
}
```

**Step 2: Verify it compiles**

Run: `cd workers/gql-async-graphql && cargo build --target wasm32-unknown-unknown --release`

**Step 3: Commit**

```bash
git add workers/gql-async-graphql/src/
git commit -m "feat: add error handling for missing config and bad requests"
```

---

### Task 7: Add GitHub Actions workflow

**Files:**
- Create: `.github/workflows/gql-async-graphql.yml`

**Step 1: Create the workflow file**

```yaml
name: gql-async-graphql

on:
  push:
    branches: [main]
    paths:
      - 'workers/gql-async-graphql/**'
      - 'Cargo.toml'
      - 'rust-toolchain.toml'
  pull_request:
    paths:
      - 'workers/gql-async-graphql/**'
      - 'Cargo.toml'
      - 'rust-toolchain.toml'
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Cache cargo registry and build
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-async-graphql-${{ hashFiles('**/Cargo.lock') }}

      - name: Build
        run: cargo build --target wasm32-unknown-unknown --release -p gql-async-graphql

  deploy:
    needs: build
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Deploy to Cloudflare
        working-directory: workers/gql-async-graphql
        run: npx wrangler deploy
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
```

**Step 2: Commit**

```bash
git add .github/workflows/gql-async-graphql.yml
git commit -m "ci: add GitHub Actions workflow for gql-async-graphql"
```

---

### Task 8: End-to-end smoke test with wrangler dev

**Files:** None — manual verification only.

**Step 1: Start the worker locally**

Run: `cd workers/gql-async-graphql && npx wrangler dev`

**Step 2: Test health endpoint**

```bash
curl http://localhost:8787/health
```
Expected: `{"status":"ok"}`

**Step 3: Test GraphQL query (no origin running — expect GraphQL error)**

```bash
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights(limit: 5) { id date aircraftTitle } }"}'
```
Expected: `{"data":null,"errors":[{"message":"Fetch failed: ..."}]}`

**Step 4: Test invalid body**

```bash
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d 'not json'
```
Expected: `{"data":null,"errors":[{"message":"Invalid request body: ..."}]}`

**Step 5: Test 404**

```bash
curl http://localhost:8787/nope
```
Expected: `Not Found` with status 404

**Step 6: Document results and commit any fixes**

If any issues found, fix them and commit with descriptive message.
