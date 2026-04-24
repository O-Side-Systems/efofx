# Phase 2D — Widget Public Endpoints

**Status:** in progress
**Branch:** `rust-port`
**Date:** 2026-04-24
**Parent:** [RUST-PORT-PLAN.md §Phase 2D](../RUST-PORT-PLAN.md#phase-2d--widget-public-endpoints)

## Goal

The widget (`apps/efofx-widget`) can complete a full demo flow against
Rust without touching FastAPI: branding → chat → estimate → lead /
consultation, with analytics reporting on the side.

The chat + estimate surface already runs on Rust (Phase 2B, 2C). This
phase closes the remaining widget-facing gaps.

## Scope (in order of ship)

| # | Surface | Auth | Notes |
|---|---|---|---|
| 2D.1 | `GET /v1/widget/branding/{prefix}` | public | 30 req/min/IP; reads `tenants.settings.branding`; falls back to `BrandingConfig::default()` |
| 2D.2 | `POST /v1/widget/leads` | widget API key | inserts into `widget_leads` |
| 2D.3 | `POST /v1/widget/consultations` | widget API key | saves lead w/ `lead_type=consultation`; fires Resend email if configured; email failure is non-fatal |
| 2D.4a | `POST /v1/widget/events` | widget API key | `$inc` upsert on `widget_analytics` daily bucket, returns 204 |
| 2D.4b | `GET /v1/widget/events` | Supabase JWT | returns last N days buckets; 10 req/min |
| 2D.5 | per-tenant CORS | layer | origins sourced from `tenants.settings.allowed_origins`, cached in-process, fed on branding fetch + widget-key auth |
| 2D.6 | contract tests + checkpoint doc | — | utoipa-generated OpenAPI as source of truth |

## Resolved decisions (2026-04-24)

1. **Public branding prefix semantics.** Match FastAPI:
   `api_key_prefix` is the **32-hex tenant_id without dashes** (the
   first 32 chars after `sk_live_` in the widget's stored API key).
   The current utoipa string ("last-6 hex") is wrong and gets
   corrected in 2D.1.
2. **Rate-limit storage.** In-memory `governor` crate only. Trait-
   abstracted (`RateLimiter`) so a Valkey-backed impl can drop in
   during Phase 4. Keyed by `(ip, route_group)` with buckets per
   configured rate.
3. **Email provider.** Direct **Resend** HTTP API, not SMTP. New
   `efofx-email` crate. Silent no-op when `email.resend_api_key` is
   absent, mirroring FastAPI's "skip if MAIL_* unset" behavior.
4. **Tenant settings schema.** Add a typed `TenantSettingsDoc`
   persisted under `tenants.settings`:
   ```rust
   struct TenantSettingsDoc {
       branding: Option<BrandingConfig>,
       allowed_origins: Vec<String>,
   }
   ```
   Read by branding fetch + CORS middleware. Writes (beyond
   provisioning defaults) are out of scope for 2D; a future
   `PATCH /v1/me` extension will expose them to the dashboard.
5. **`/v1/widget/events` two-verb routing.** Keep the same path with
   separate middleware stacks — `POST` on `widget_api_key`, `GET` on
   `supabase_jwt`. Same pattern as `/v1/me` today.

## Collections touched

| Collection | Shape | Indexes |
|---|---|---|
| `tenants` | add optional `settings: { branding, allowed_origins }` subdoc | existing `tenant_id` unique index suffices |
| `widget_leads` | `{ tenant_id, session_id, name, email, phone, lead_type?, consultation_message?, captured_at }` | `{ tenant_id: 1, captured_at: -1 }` |
| `widget_analytics` | `{ tenant_id, date, widget_view, chat_start, estimate_complete }` | `{ tenant_id: 1, date: 1 }` unique |

FastAPI parity: field names and daily bucket semantics match
`apps/efofx-estimate/app/services/widget_service.py`.

## Files expected to land

New:
- `crates/efofx-core/src/middleware/mod.rs`
- `crates/efofx-core/src/middleware/rate_limit.rs`
- `crates/efofx-core/src/middleware/tenant_cors.rs`
- `crates/efofx-email/` (new crate — `EmailSender` trait, Resend impl, Noop impl)
- `crates/efofx-storage/src/widget.rs` (`WidgetLeadRepo`, `WidgetAnalyticsRepo`)
- `crates/efofx-core/tests/widget_*.rs`
- `docs/rust-port/phase-2d-checkpoint.md`

Modified:
- `crates/efofx-storage/src/tenants.rs` — add `settings` to `TenantDoc`, expose read/update helpers
- `crates/efofx-storage/src/lib.rs` — re-export new repos
- `crates/efofx-config/src/lib.rs` — add `EmailConfig`, `WidgetConfig`, `RateLimitConfig`
- `crates/efofx-core/src/api/widget.rs` — swap `not_implemented` stubs for real handlers
- `crates/efofx-core/src/lib.rs` — wire new state + middleware
- `crates/efofx-core/src/openapi.rs` — no new paths; already present from Phase 1.2

## Definition of Done

- Widget pointed at Rust completes the full demo flow (branding →
  chat → estimate → lead) without FastAPI in the path.
- Rate limits reject bursts with a clean `{errors:[…]}` envelope and
  `429`.
- Consultation email silently skips when Resend is unconfigured; lead
  is still persisted.
- Per-tenant CORS allows the tenant's configured origins and rejects
  others for widget-auth routes.
- Contract tests pass against utoipa-generated OpenAPI.
- `cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace` green.

## Non-goals for 2D

- Dashboard UI for managing branding / allowed origins (future).
- Valkey-backed rate limiter (Phase 4).
- Tenant-scoped CRUD for branding / origins (`PATCH /v1/me` extension — future).
- Feedback, calibration, contractor routing (Phases 2E, 2F).
