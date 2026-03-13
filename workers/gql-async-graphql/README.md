# gql-async-graphql

GraphQL flight log API on Cloudflare Workers, built with [`async-graphql`](https://github.com/async-graphql/async-graphql) and [`workers-rs`](https://github.com/cloudflare/workers-rs).

## Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target
- Node.js (for `wrangler` CLI)
- An origin API running on `http://localhost:8080`

```sh
rustup target add wasm32-unknown-unknown
```

## Run locally

Make sure Cargo is available in your shell:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Start the upstream origin API first. This worker reads `ORIGIN_BASE_URL` from `wrangler.toml`,
which defaults to `http://localhost:8080`.

Then start the worker from `workers/gql-async-graphql`:

```sh
npx wrangler dev
```

This starts the worker on `http://localhost:8787`. The origin API URL is configured via `ORIGIN_BASE_URL` in `wrangler.toml`.

### Endpoints

| Method | Path       | Description                |
|--------|------------|----------------------------|
| GET    | /health    | Health check               |
| POST   | /graphql   | GraphQL endpoint           |

### Example queries

```sh
# Health check
curl http://localhost:8787/health

# List flights
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights(limit: 5) { id date aircraftTitle } }"}'

# Get a single flight
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flight(id: \"abc\") { id date departureIcao arrivalIcao } }"}'
```

If `GET /health` succeeds but GraphQL requests fail, check that the upstream origin API is
running on `http://localhost:8080` or change `ORIGIN_BASE_URL` in `wrangler.toml`.

## Run tests

Native unit tests run on the host, not in WASM. Run them from the workspace root:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -p gql-async-graphql
```

## Build for WASM

```sh
cargo build --target wasm32-unknown-unknown --release -p gql-async-graphql
```

## Deploy

Cloudflare infrastructure for this worker is managed by Terraform in
`infra/terraform/cloudflare`. That stack creates the Worker shell and configures `workers.dev`.
Wrangler is still used to upload the Worker code itself.

If you want to protect the `workers.dev` hostname with Cloudflare Access, enable that in the
Cloudflare dashboard after the Worker exists.

```sh
cd workers/gql-async-graphql
npx wrangler deploy
```

Requires `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` environment variables (or `wrangler login`).

In CI, Terraform apply runs before deploy on non-PR executions. The deploy step itself remains
disabled, so the current automated rollout path is infrastructure-only.
