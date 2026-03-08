# ADR 001: Target Platform — Cloudflare Workers

**Date:** 2026-03-08
**Status:** Accepted

## Context

We need an edge compute platform for deploying a Rust/WASM GraphQL server. The primary candidates are Cloudflare Workers and Fastly Compute.

## Decision

Target Cloudflare Workers as the deployment platform.

## Rationale

- **Pricing:** $5/month paid plan includes 10M requests and 30M CPU-ms. Overage at $0.30/M requests and $0.02/M CPU-ms. Free egress. Fastly's pricing is opaque and generally higher.
- **Network:** 300+ edge locations vs Fastly's ~90.
- **Ecosystem:** Richer storage options (KV, D1, R2, Durable Objects, Hyperdrive) for future middleware needs (rate limiting counters, caching).
- **Transparency:** Fully public pricing model vs Fastly's "contact sales" approach.

Fastly shows better raw TTFB performance (~2x in NA/EU benchmarks), but for a GraphQL API where response assembly dominates, this is less significant.

## Consequences

- Build toolchain uses `wrangler` CLI
- WASM target is `wasm32-unknown-unknown` (via `workers-rs`)
- Architecture should remain portable — core logic has no Cloudflare dependencies, enabling future migration to Fastly or generic WASI
