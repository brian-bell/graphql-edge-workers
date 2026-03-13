# ADR 007: Build & Deploy Toolchain

**Date:** 2026-03-08
**Status:** Accepted

## Context

Rust + WASM targeting Cloudflare Workers requires a specific toolchain. We need reproducible builds locally and in CI.

## Decision

Use the standard Cloudflare Workers Rust toolchain:

| Tool | Purpose |
|---|---|
| rustup | Installs Rust + `wasm32-unknown-unknown` target |
| cargo | Build tool, manages workspace and dependencies |
| wrangler | Cloudflare CLI — builds, local dev, deploys |
| worker-build | Compiles Rust → WASM, generates JS glue (invoked by wrangler) |

Pin Rust version via `rust-toolchain.toml` at workspace root.

## CI/CD

Two independent GitHub Actions workflows (one per worker):

1. Trigger: push to `main` with changes in that worker's directory, or manual dispatch
2. Detect changed paths and split worker build/test from Terraform infrastructure work
3. Build/test in a prebuilt CI container image with Rust, the WASM target, Node/npm, and Terraform preinstalled
4. Run Terraform in a separate job attached to the `cloudflare` GitHub environment only when the Terraform stack changes, or on manual dispatch
5. Run Terraform apply before deploy on non-PR executions
6. Keep `wrangler deploy` present but disabled until code deployment is intentionally enabled

Required GitHub environment secrets in `cloudflare`:

- `CLOUDFLARE_API_TOKEN`
- `CLOUDFLARE_ACCOUNT_ID`
- `CLOUDFLARE_WORKERS_DEV_SUBDOMAIN`
- `R2_STATE_BUCKET`
- `R2_STATE_ACCESS_KEY_ID`
- `R2_STATE_SECRET_ACCESS_KEY`

## Rationale

- No external build system needed — wrangler orchestrates the full chain
- `rust-toolchain.toml` ensures CI and local builds use the same compiler
- Path-filtered workflows avoid deploying one worker when only the other changed
- Manual dispatch enables ad-hoc deploys
- A prebuilt CI image avoids reinstalling Rust, Terraform, and Node on every run
- Terraform gives the project declarative control over Cloudflare Worker infrastructure
- Cloudflare R2 keeps shared Terraform state inside the same provider footprint as the application

## Consequences

- Developers need rustup, wrangler, and Node.js (wrangler dependency) installed locally
- CI now depends on a published GHCR image for the worker workflow container
- The Terraform backend must be bootstrapped out of band before CI apply can work
- `workers.dev` Cloudflare Access remains a manual dashboard configuration rather than a Terraform-managed resource
- Each worker deploys independently — no coordinated releases needed
