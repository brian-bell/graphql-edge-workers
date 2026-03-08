# ADR 005: Custom Parser — Partial GraphQL Spec

**Date:** 2026-03-08
**Status:** Accepted

## Context

A full GraphQL spec-compliant parser is a substantial undertaking. The custom parser implementation needs enough spec coverage to be a realistic GraphQL endpoint, but scoped enough to be a reasonable comparison project.

## Decision

Implement partial GraphQL spec compliance:

**Supported:**
- Fields (nested selection sets)
- Arguments (literal values and variable references)
- Variables (declared in operation, passed via JSON)
- Aliases (field renaming)

**Not supported:**
- Fragments (inline or named)
- Directives (@skip, @include, custom)
- Introspection (__schema, __type)
- Subscriptions

## Rationale

- Covers the features needed for real client queries (Postman, Insomnia, frontend apps)
- Variables + aliases make it a proper GraphQL endpoint, not just a pattern matcher
- Fragments and directives add significant complexity with limited value for a small fixed schema
- Introspection is a large surface area and not needed for a controlled comparison

## Consequences

- Clients cannot use fragments in queries sent to the custom parser
- No tooling auto-discovery (no introspection) — clients need the schema out-of-band
- The parser can be extended later if the approach proves worthwhile
