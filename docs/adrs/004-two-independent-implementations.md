# ADR 004: Two Independent Implementations

**Date:** 2026-03-08
**Status:** Accepted

## Context

We want to evaluate whether using a full-featured GraphQL library is the right approach for an edge worker, or whether a lightweight custom parser is more appropriate given the constrained environment (WASM binary size, cold starts, CPU limits).

## Decision

Build two fully independent implementations of the same GraphQL server:

1. **gql-async-graphql** — Uses the `async-graphql` crate (macro-driven, full spec support)
2. **gql-custom-parser** — Hand-written lexer/parser/executor (partial spec)

Both serve the same schema, call the same upstream API, and deploy as separate Cloudflare Workers.

## Rationale

- Direct comparison on the metrics that matter for edge: binary size, cold start, latency
- Also reveals developer ergonomics trade-offs (boilerplate, maintainability, ease of schema changes)
- Building both is feasible because the schema is small and resolvers are simple HTTP calls

## Consequences

- Each implementation gets its own implementation plan
- No shared code between them — comparison must be fair
- Same schema, same upstream contract, same deployment model
- Comparison metrics: binary size, cold start, request latency, DX, LOC, compile time
