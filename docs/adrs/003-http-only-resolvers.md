# ADR 003: HTTP-Only Resolvers

**Date:** 2026-03-08
**Status:** Accepted

## Context

The data lives in Postgres on DigitalOcean. Cloudflare Workers cannot make raw TCP connections to Postgres. Options considered:

1. **Cloudflare Hyperdrive** — Built-in Postgres proxy, but requires JS-based bindings
2. **HTTP-based Postgres proxy** — PostgREST or similar in front of Postgres
3. **Serverless Postgres provider** — Neon/Supabase (requires migration off DigitalOcean)
4. **Thin origin API service** — Small Rust service on DigitalOcean next to Postgres, exposing an HTTP API

## Decision

Option 4: Resolvers in the edge worker only make HTTP calls to an upstream origin API. No direct database access from the edge.

## Rationale

- Clean separation of concerns — edge worker handles GraphQL parsing/execution, origin handles data access
- No vendor lock-in on the data layer
- Origin service can be built independently (e.g., Axum + sqlx)
- Keeps the edge worker simple and focused

## Consequences

- Both worker implementations use `worker::Fetch` for upstream calls
- Origin base URL is configured via environment variable (`ORIGIN_BASE_URL`)
- Origin API is out of scope for this project but assumed to exist
- Upstream API contract: REST endpoints (`GET /flights/1`, `GET /flights?limit=10&offset=0`, `POST /flights`)
