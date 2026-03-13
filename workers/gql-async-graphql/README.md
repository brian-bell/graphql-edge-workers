# gql-async-graphql

GraphQL flight log API on Cloudflare Workers, built with [`async-graphql`](https://github.com/async-graphql/async-graphql) and [`workers-rs`](https://github.com/cloudflare/workers-rs).

## Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target
- Node.js (for `wrangler` CLI)
- A Supabase project with:
  - `SUPABASE_URL`
  - `SUPABASE_PUBLISHABLE_KEY`
  - Auth enabled for your client
  - A `flights` table exposed through PostgREST

```sh
rustup target add wasm32-unknown-unknown
```

## Run locally

Make sure Cargo is available in your shell:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Set `SUPABASE_URL` and `SUPABASE_PUBLISHABLE_KEY` in `wrangler.toml` (or override them with your local env/secrets as appropriate).

Then start the worker from `workers/gql-async-graphql`:

```sh
npx wrangler dev
```

This starts the worker on `http://localhost:8787`.

### Endpoints

| Method | Path       | Description                |
|--------|------------|----------------------------|
| GET    | /health    | Health check               |
| POST   | /graphql   | Authenticated GraphQL endpoint |

### Example queries

```sh
# Health check
curl http://localhost:8787/health

# List flights with a Supabase access token
export SUPABASE_ACCESS_TOKEN="your-user-access-token"

curl -X POST http://localhost:8787/graphql \
  -H "Authorization: Bearer $SUPABASE_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights(limit: 5) { id date aircraftTitle } }"}'

# Get a single flight
curl -X POST http://localhost:8787/graphql \
  -H "Authorization: Bearer $SUPABASE_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flight(id: \"abc\") { id date departureIcao arrivalIcao } }"}'
```

`POST /graphql` requires a valid Supabase access-token JWT. The Worker verifies the token against the project's JWKS and forwards the same bearer token to Supabase so RLS applies.

The worker expects the `flights` table to use a `user_id uuid not null` column for ownership. A starter SQL policy file lives at `workers/gql-async-graphql/supabase/rls.sql`.

## Service-to-service auth

For daemon or backend services that need to call the GraphQL API (e.g. a Python service running as a Windows daemon), use a dedicated Supabase user account:

1. Create a service account user in your Supabase project (e.g. `daemon@yourorg.internal` with a strong password).
2. On startup, sign in via the Supabase Auth REST API:
   ```
   POST {SUPABASE_URL}/auth/v1/token?grant_type=password
   ```
   with the service account's email and password to obtain an access token and refresh token.
3. Use the access token as `Authorization: Bearer <token>` for all GraphQL requests.
4. Refresh the token before it expires (default 1 hour) using the refresh token:
   ```
   POST {SUPABASE_URL}/auth/v1/token?grant_type=refresh_token
   ```

This approach requires no changes to the worker. The service account's JWT validates through the same JWKS flow as interactive users, and RLS scopes data to the service account's `user_id`.

Store the service account credentials securely (e.g. Windows Credential Manager, environment variables, or a secrets manager). Do not hard-code them.

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
`infra/terraform/cloudflare`. That stack creates the Worker shell and protects the `workers.dev`
hostname with Cloudflare Access. Wrangler is still used to upload the Worker code itself.

```sh
cd workers/gql-async-graphql
npx wrangler deploy
```

Requires `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` environment variables (or `wrangler login`).

In CI, Terraform apply runs before deploy on non-PR executions. The deploy step itself remains
disabled, so the current automated rollout path is infrastructure-only.
