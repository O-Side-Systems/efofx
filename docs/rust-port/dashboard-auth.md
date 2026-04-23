# Dashboard Auth Integration (Phase 2A)

**Status:** guide only — no dashboard code changes ship in Phase 2A. This document exists so Phase 3 cutover is unblocked.

## Model

- **Supabase** owns every credential operation: sign-up, sign-in, email verification, password reset, session refresh. We never see the password, and we never store it.
- **Rust core** only verifies the JWT that Supabase issues and maps its `sub` claim to the tenant it provisioned on first seen.

A user signs in via `@supabase/supabase-js`. The SDK returns a session with an `access_token` (JWT). The dashboard attaches that token as `Authorization: Bearer <jwt>` on every Rust API call.

## Minimal dashboard client

```ts
import { createClient } from '@supabase/supabase-js';

const supabase = createClient(
  import.meta.env.VITE_SUPABASE_URL,
  import.meta.env.VITE_SUPABASE_ANON_KEY,
);

// Sign-up — company_name is captured as user_metadata so the Rust
// provisioner can use it instead of falling back to email.
export async function signUp(email: string, password: string, companyName: string) {
  return supabase.auth.signUp({
    email,
    password,
    options: {
      data: { company_name: companyName },
    },
  });
}

// Sign-in.
export async function signIn(email: string, password: string) {
  return supabase.auth.signInWithPassword({ email, password });
}

// Call an authenticated Rust endpoint.
export async function fetchMe() {
  const { data: { session } } = await supabase.auth.getSession();
  if (!session) throw new Error('not signed in');

  const resp = await fetch(`${import.meta.env.VITE_CORE_API_URL}/v1/me`, {
    headers: { Authorization: `Bearer ${session.access_token}` },
  });
  if (!resp.ok) throw new Error(`core returned ${resp.status}`);
  return resp.json();
}
```

## Auto-refresh

`@supabase/supabase-js` refreshes the access token ~60 seconds before expiry by default. Wrap `fetch` in a thin helper that retrieves the current session on every call — never cache the JWT in a long-lived variable:

```ts
async function coreFetch(path: string, init?: RequestInit) {
  const { data: { session } } = await supabase.auth.getSession();
  const headers = new Headers(init?.headers);
  headers.set('Authorization', `Bearer ${session!.access_token}`);
  return fetch(`${import.meta.env.VITE_CORE_API_URL}${path}`, { ...init, headers });
}
```

## First-login side effects

The very first authenticated call after sign-up triggers tenant provisioning in the Rust core (see `efofx-storage::auth::TenantResolver::resolve_or_provision`). The response includes the full `Tenant` record. If `GET /v1/me` returns a `tenant.not_found` error envelope, the JWT was accepted but the upsert failed — treat as transient and retry once.

## Error mapping

| HTTP status  | Body `code`                         | UI behaviour                    |
|--------------|-------------------------------------|---------------------------------|
| 401          | `auth.missing_token`                | Redirect to sign-in             |
| 401          | `auth.invalid_token`                | Force sign-out + re-auth        |
| 401          | `auth.expired_token`                | Call `supabase.auth.refreshSession()` then retry |
| 401          | `auth.wrong_audience`               | Config mismatch — surface to ops|
| 404          | `tenant.not_found`                  | Transient provisioning race — retry |

Every error returns the standard envelope (see `efofx-openapi::ApiError`), so a single mapper covers the whole API surface.

## Env vars the dashboard needs

- `VITE_SUPABASE_URL` — same project as the Rust core's `supabase.url`
- `VITE_SUPABASE_ANON_KEY` — the public anon key (not the service_role key)
- `VITE_CORE_API_URL` — base URL for the Rust core (e.g. `https://api.efofx.com`)

The Rust core never needs the `service_role` key and must not be given one.

## Widget is separate

The widget does **not** sign users in. It authenticates with a long-lived per-tenant API key (`sk_live_...`) issued via `POST /v1/me/api-keys:rotate`. Widget integration is documented in the widget's own README once Phase 2D lands.
