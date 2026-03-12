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

The `gql-async-graphql` workflow now:

1. Detects whether the change set affects worker build paths or Terraform paths
2. Runs the Rust build job in a prebuilt CI container with Rust, the WASM target, Node/npm, and Terraform installed
3. Runs the Terraform job separately, attached to the `cloudflare` GitHub environment
4. Only runs the Terraform job for `infra/terraform/cloudflare/**` changes or manual dispatch
5. Runs remote-state Terraform plan steps and PR comments when secrets are available
6. Runs `terraform apply` before deploy on non-PR Terraform runs
7. Keeps `wrangler deploy` disabled for now

The CI container image is defined at `.github/docker/gql-async-graphql-ci/Dockerfile`
and published by `.github/workflows/ci-image-gql-async-graphql.yml`.

If the image does not exist yet, run the image workflow once before expecting the worker workflow
to succeed.

## Key Constraints

- Each worker crate is fully independent — no shared crates or cross-dependencies
- Resolvers only make HTTP calls to the origin API, no direct DB access
- See `docs/adrs/` for all architectural decisions
- See `docs/plans/` for implementation plans
