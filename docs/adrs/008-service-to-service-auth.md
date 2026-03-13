# ADR 008: Service-to-Service Auth via Supabase User Accounts

**Date:** 2026-03-12
**Status:** Accepted

## Context

Backend services (e.g. a Python daemon running on Windows) need to call the GraphQL API without an interactive user session. We need a pattern that works with the existing JWT auth and RLS enforcement without requiring changes to the worker.

## Decision

Use a dedicated Supabase user account per service:

1. Create a service account user in the Supabase project (e.g. `daemon@yourorg.internal` with a strong password).
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

The service account's JWT validates through the same JWKS flow as interactive users, and RLS scopes data to the service account's `user_id`.

## Consequences

- No worker changes needed — service accounts are regular Supabase users
- Each service gets its own `user_id`, so RLS isolates its data from other users
- Service credentials must be stored securely (e.g. OS credential manager, environment variables, or a secrets vault) — never hard-coded
- Token refresh logic must be implemented by the calling service
