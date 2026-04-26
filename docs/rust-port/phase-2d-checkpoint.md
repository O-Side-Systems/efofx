# Phase 2D Checkpoint — Widget public endpoints

**Date:** 2026-04-25
**Branch:** `rust-port`
**Status:** All 6 subphases committed. Phase 2D closed.

---

## Goal

The widget (`apps/efofx-widget`) can complete the full demo flow against
Rust without touching FastAPI: branding → chat → estimate → lead /
consultation, with analytics reporting on the side.

## What shipped

| # | Commit | Scope |
|---|---|---|
| 2D.1 | `07198c7` | `GET /v1/widget/branding/{prefix}` + IP rate limiter |
| 2D.2 | `2323dce` | `POST /v1/widget/leads` |
| 2D.3 | `fb492ad` | `POST /v1/widget/consultations` + `efofx-email` crate (Resend + Noop) |
| 2D.4 | `62e8f6a` | `WidgetAnalyticsRepo` + `POST/GET /v1/widget/events` |
| 2D.5 | `4075e1c` | Per-tenant CORS layer (`OriginCache`, predicate, seeding paths) |
| 2D.6 | _this commit_ | Contract tests + `:name` route fix + checkpoint doc |

Run `git log --oneline rust-port ^main` to see all Phase 2 commits.

## Surface summary

| Verb | Path | Auth | Limits |
|---|---|---|---|
| GET | `/v1/widget/branding/{prefix}` | public | 30 req/min/IP |
| POST | `/v1/widget/leads` | widget API key | per-tenant CORS |
| POST | `/v1/widget/consultations` | widget API key | per-tenant CORS, Resend best-effort |
| POST | `/v1/widget/events` | widget API key | per-tenant CORS, fire-and-forget |
| GET | `/v1/widget/events` | Supabase JWT | 10 req/min/IP |

## Decisions locked during 2D

1. **Public branding prefix semantics** — 32-hex tenant_id (no dashes),
   the segment after `sk_live_` in the widget's stored API key. Matches
   FastAPI's `api_key_prefix`.
2. **Rate-limit storage** — process-local `governor`. Trait-shaped
   (`IpRateLimiter`) so a Valkey-backed impl can drop in during Phase 4.
3. **Email provider** — Direct Resend HTTP API via `efofx-email` crate.
   Silent no-op (`NoopSender`) when `email.resend_api_key` is absent.
   Lead-side is the source of truth; email failure never fails the response.
4. **Tenant settings schema** — `TenantSettingsDoc` under
   `tenants.settings`: `{ branding, allowed_origins }`. Read by branding
   fetch + per-tenant CORS. Writes are deferred to a future
   `PATCH /v1/me` extension.
5. **`/v1/widget/events` two-verb routing** — same path, separate
   middleware stacks. POST on `widget_api_key`, GET on `supabase_jwt`.
6. **Per-tenant CORS** — runtime-mutable `OriginCache` with the
   `tower_http::CorsLayer` predicate consulting it. Seeded by the
   branding handler (primary, every widget bootstrap) and the post-auth
   `seed_widget_origins` middleware (cache-miss only, defense in depth).
7. **Global `Any/Any/Any` CorsLayer removed** — it was overriding the
   per-tenant layer. Other surfaces (dashboard, health, openapi) are
   server-to-server in the demo deployment, so no global CORS is needed.

## Files added in 2D

- `crates/efofx-core/src/middleware/rate_limit.rs`
- `crates/efofx-core/src/middleware/tenant_cors.rs`
- `crates/efofx-email/` (new crate)
- `crates/efofx-storage/src/widget.rs` (`WidgetLeadRepo`,
  `WidgetAnalyticsRepo`, `LeadDoc`, `WidgetDailyBucket`)
- `crates/efofx-core/tests/widget_branding.rs`
- `crates/efofx-core/tests/widget_leads.rs`
- `crates/efofx-core/tests/widget_consultation.rs`
- `crates/efofx-core/tests/widget_events.rs`
- `crates/efofx-core/tests/widget_cors.rs`

## Files modified in 2D

- `crates/efofx-storage/src/tenants.rs` — `TenantSettingsDoc`,
  `BrandingWithOrigins`, `fetch_branding_by_prefix`,
  `fetch_allowed_origins`
- `crates/efofx-config/src/lib.rs` — `EmailConfig`
- `crates/efofx-core/src/api/widget.rs` — handlers replace `not_implemented` stubs
- `crates/efofx-core/src/lib.rs` — wires `widget_analytics`,
  `origin_cache`, `email`, `email_from`; removes global CorsLayer
- `crates/efofx-core/src/openapi.rs` — `openapi_widget_surface_contract` snapshot test
- `crates/efofx-core/tests/common/mod.rs` — `TestHarness` exposes
  `origin_cache` for CORS contract tests

## Bug found & fixed during 2D.6 — axum 0.7 path-param syntax

The branding route was registered as `/v1/widget/branding/{api_key_prefix}`.
matchit 0.7 (axum 0.7's router backend) only accepts `:name` syntax —
the `{...}` form was treated as a literal, so the route never matched.
2D.6 contract tests caught this; fix is `:api_key_prefix` for the route
registration. The OpenAPI doc keeps `{name}` (the OpenAPI 3 standard).

**Other routes still using `{name}` and known to be broken:**

- `GET /v1/chat/sessions/{session_id}` (Phase 2B)
- `POST /v1/chat/sessions/{session_id}/messages` (Phase 2B)
- `POST /v1/chat/sessions/{session_id}:generate-estimate` (Phase 2C.6)
- `GET /v1/estimates/{session_id}` — fixed in this commit
- `GET/PATCH /v1/leads/{lead_id}` (future phase, currently a 501 stub)
- `GET/POST /v1/feedback/forms/{token}` (future phase, currently 501 stubs)

The chat `:generate-estimate` route is the one structural blocker —
matchit 0.7 cannot express `:name:literal-suffix`, so the demo's
chat→estimate flow needs either a URL change to
`/v1/chat/sessions/:session_id/generate-estimate` (matches FastAPI
exactly) or an axum 0.8 upgrade. Tracked as a follow-up; not on the
2D demo path.

## Definition of Done — status

- [x] Widget can complete branding → lead / consultation flow against Rust
- [x] Rate limits reject bursts with the documented `{errors:[…]}` envelope
- [x] Consultation email skips silently when Resend is unconfigured
- [x] Per-tenant CORS allows configured origins, rejects others
- [x] `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
      `cargo test --workspace` all green
- [ ] **Live integration test** against a real Mongo + a seeded tenant
      (deferred to Phase 4 alongside the Valkey + Resend live keys)

## Test coverage

| Suite | Coverage |
|---|---|
| `widget_branding.rs` | Malformed-prefix 404 + non-hex 404 (no Mongo) |
| `widget_leads.rs` | Auth-rejection envelope (missing + bogus key) |
| `widget_consultation.rs` | Auth-rejection envelope |
| `widget_events.rs` | POST + GET auth-rejection envelopes |
| `widget_cors.rs` | Preflight allow/deny + actual-request header reflection |
| `openapi::tests::openapi_widget_surface_contract` | Pinned status codes + auth schemes + schema components |
| `efofx-email` unit tests | Noop, Resend success, Resend rejection |
| `tenant_cors::tests` | OriginCache insert / contains / dedup |

Persistence + happy-path coverage (201 with valid API key, 200 branding
with valid tenant) lands in the infrastructure suite (Phase 4).

## Follow-ups queued for later phases

- **Axum 0.8 upgrade** or per-route URL changes to fix `{}` path-param
  routes and the `:generate-estimate` pseudo-RPC URL
- `PATCH /v1/me` extension to expose branding + allowed_origins editing
- Origin cache eviction on revoke (today: revoked origins remain
  allowed until process restart)
- Live OpenAPI byte-compat check vs. the FastAPI-generated doc
- Valkey-backed rate limiter
- Dashboard read path for leads (`GET /v1/leads`, `PATCH /v1/leads/{id}`)
