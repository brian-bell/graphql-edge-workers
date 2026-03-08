# ADR 006: Monorepo With No Shared Code

**Date:** 2026-03-08
**Status:** Accepted

## Context

Two independent implementations need a project structure. Options:

1. **Monorepo with shared core crate** — common types/HTTP client, both depend on it
2. **Separate repositories** — maximum isolation, duplicated boilerplate
3. **Monorepo, no shared code** — single Cargo workspace, independent crates

## Decision

Option 3: Single Cargo workspace with two independent crates. No shared dependency crate.

## Rationale

- Preserves full independence for a fair comparison
- Single repo makes side-by-side comparison, single commit history, and single CI config easy
- Cargo workspace handles build orchestration without coupling
- If shared patterns emerge later, extracting a common crate is straightforward within a workspace

## Consequences

- Some boilerplate duplication (HTTP client setup, error types, Flight struct)
- Each crate has its own `Cargo.toml` and `wrangler.toml`
- Two independent GitHub Actions workflows (path-filtered)
- `Cargo.toml` at root defines workspace members only
