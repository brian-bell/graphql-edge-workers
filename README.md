# Rust GraphQL on Cloudflare Workers

Comparative study of two GraphQL server implementations in Rust targeting Cloudflare Workers (WASM). Once evaluated, the chosen approach becomes the production server. Both serve a flight log schema and resolve data via HTTP calls to an upstream origin API.

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

## Local Development

The `gql-async-graphql` worker currently has the only implemented local service flow in this repo.
It serves on `http://localhost:8787` and proxies resolver requests to an upstream origin API at
`http://localhost:8080`.

Start the worker from the workspace root:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cd workers/gql-async-graphql
npx wrangler dev
```

Verify the worker:

```sh
curl http://localhost:8787/health

curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ flights(limit: 5) { id date aircraftTitle } }"}'
```

Run native tests from the workspace root:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -p gql-async-graphql
```

## Infrastructure

The `gql-async-graphql` worker now has a Terraform stack at
`infra/terraform/cloudflare`.

Terraform owns:

- the Cloudflare Worker shell and `workers.dev` enablement
- Worker observability settings
- a Cloudflare Access application and allow policy protecting the `workers.dev` hostname

Wrangler still owns Worker code deployment.

Shared Terraform state is stored in Cloudflare R2 through the Terraform `s3` backend.
See `infra/terraform/cloudflare/README.md` for the exact local bootstrap flow.

## CI/CD

The `gql-async-graphql` workflow runs a single build job in a prebuilt CI container that runs
`cargo test` and builds the worker for `wasm32-unknown-unknown`. It triggers on worker source and
build-input changes, and on manual dispatch.

CI does not run Terraform or deploy to Cloudflare; both are done locally.

The CI container image is defined at `.github/docker/gql-async-graphql-ci/Dockerfile`
and published by `.github/workflows/ci-image-gql-async-graphql.yml`.

If the image does not exist yet, run the image workflow once before expecting the worker workflow
to succeed.

## Key Constraints

- Each worker crate is fully independent — no shared crates or cross-dependencies
- Resolvers only make HTTP calls to the origin API, no direct DB access
- See `docs/adrs/` for all architectural decisions
- See `docs/plans/` for implementation plans
