# Efofx Codebase State

**Last Updated:** 2026-07-07

> **2026-07-07 (quality pass):** A post-cutover review fixed three
> correctness bugs on the `rust-port` line: (1) chat and leads routes
> were registered with axum 0.8 `{param}` syntax, which axum 0.7 treats
> as literals — the widget's chat → estimate flow 404'd; now `:param`
> with route-matching regression tests. (2) `PATCH /v1/me` accepted a
> `settings` payload but silently dropped it; settings (branding,
> allowed_origins, routing) now validate and persist per-subfield.
> (3) `render_directory_url` corrupted multi-byte UTF-8 in partner
> directory templates. Known deferrals (leads 501 stubs, EstimationOutput
> persistence, settings echo on `GET /v1/me`, per-tenant LLM quotas)
> are tracked in `apps/efofx-core/README.md` under Phase status.
**Purpose:** Living reference for any developer or AI agent working in this codebase. Update this document as the codebase evolves.

> **2026-04-28:** Phase 3 of the Rust port (`docs/RUST-PORT-PLAN.md`,
> `docs/rust-port/phase-3-plan.md`) cut both clients over to the Rust
> core and decommissioned the FastAPI service. The authoritative
> backend is now **`apps/efofx-core`**. The original FastAPI app lives
> at `apps/archive/efofx-estimate-fastapi/` for historical reference
> only — it is not built, tested, or deployed. Sections below describe
> the post-cutover state.

---

## Repository Structure

```
efofx-workspace/              # npm workspaces monorepo
  apps/
    efofx-core/               # Rust backend (Cargo workspace) — authoritative
    efofx-widget/             # Embeddable React chat widget
    efofx-dashboard/          # Tenant dashboard (React)
    estimator-mcp-functions/  # DigitalOcean serverless MCP functions (Node.js)
    estimator-project/        # Legacy reference implementation (unused)
    synthetic-data-generator/ # Reference class seed data scripts
    archive/
      efofx-estimate-fastapi/ # Decommissioned FastAPI backend — read-only
  packages/
    efofx-shared/             # Python helpers — only consumed by the FastAPI archive
    efofx-ui/                 # Shared React components
  docs/                       # Living documentation (source of truth)
    rust-port/                # Rust port phase plans + checkpoints
    archive/                  # Historical docs — NOT authoritative
  scripts/                    # Utility scripts (key generation)
  STANDARDS.md                # Code quality standards
```

---

## 1. efofx-core (Rust Backend)

**Path:** `apps/efofx-core/`
**Stack:** Rust (1.90), axum 0.7, tokio, mongodb, jsonwebtoken (Supabase JWKS), reqwest
**Status:** Authoritative backend. Phase 2 complete; Phase 3 cuts clients
over and decommissions FastAPI in the same phase.

### Cargo workspace layout

| Crate | Role |
|-------|------|
| `efofx-core` | HTTP server (axum), route registration, app bootstrap |
| `efofx-auth` | Supabase JWT verification + JWKS cache, widget API key auth, `TenantContext` |
| `efofx-cache` | LLM response cache (Valkey/Redis-compatible) |
| `efofx-config` | Figment-based config loader: `config/efofx-core.toml` + `EFOFX_*` env |
| `efofx-crypto` | HKDF-SHA256 + AES-GCM for BYOK key envelope encryption |
| `efofx-domain` | Domain types — chat, estimation, tenant, widget, feedback |
| `efofx-email` | Resend HTTP client + no-op fallback sender |
| `efofx-llm` | OpenAI client (chat completions + structured outputs + SSE streaming) |
| `efofx-openapi` | OpenAPI 3.1 schema + contract test surface |
| `efofx-prompts` | Versioned JSON prompt registry (immutable) |
| `efofx-rcf` | Reference class matching engine |
| `efofx-storage` | MongoDB repos (tenants, sessions, estimates, leads, feedback, calibration) |

### API endpoints

All paths are under `/v1`. Auth column: `JWT` = Supabase JWT in
`Authorization: Bearer`, `API key` = widget key in `x-api-key`.

| Route | Method | Auth | Description |
|-------|--------|------|-------------|
| `/v1/me` | GET/PATCH | JWT | Tenant profile (provisions on first call via `TenantResolver`) |
| `/v1/me/openai-key` | POST/DELETE | JWT | BYOK key store/clear (validated via OpenAI before persist) |
| `/v1/me/openai-key/status` | GET | JWT | BYOK presence check |
| `/v1/me/api-keys:rotate` | POST | JWT | Mint/rotate widget API key (`sk_live_...`) |
| `/v1/chat/sessions` | POST | API key | Create chat session (optionally with `initial_message`) |
| `/v1/chat/sessions/{id}` | GET | API key | Session state + history |
| `/v1/chat/sessions/{id}/messages` | POST | API key | Append user message, get assistant follow-up |
| `/v1/chat/sessions/{id}:generate-estimate` | POST | API key | SSE: `thinking` → `estimate` → narrative tokens → `done` (incl. `routing_tags`) |
| `/v1/widget/branding/{api_key_prefix}` | GET | None (rate-limited) | Public branding fetch |
| `/v1/widget/leads` | POST | API key | Lead capture |
| `/v1/widget/consultations` | POST | API key | Consultation request + Resend email |
| `/v1/widget/events` | POST/GET | API key | Widget analytics |
| `/v1/feedback/*` | various | mixed | Outcome feedback ingest + magic-link forms |
| `/v1/calibration/metrics` | GET | JWT | Calibration accuracy aggregate |
| `/v1/calibration/trend` | GET | JWT | Calibration time-series |
| `/v1/integration/contractor-match` | POST | API key | Partner-routing handler (Phase 2F) |
| `/health` | GET | None | Health check |

### Auth model

- **Supabase** owns every credential operation (sign-up, sign-in,
  refresh). The Rust core never sees a password.
- **JWT verification** — `efofx-auth::jwks` fetches and caches
  Supabase's RSA JWKS document (`{url}/auth/v1/.well-known/jwks.json`),
  refreshed every 15 min by default.
- **First-login provisioning** — `TenantResolver::resolve_or_provision`
  upserts a tenant document keyed by the JWT `sub` claim on the first
  authenticated call.
- **Widget auth** — long-lived per-tenant API keys (`sk_live_<prefix>_<secret>`)
  minted via `POST /v1/me/api-keys:rotate`. Verified in `efofx-auth`
  via prefix lookup + constant-time secret compare against the stored
  Argon2id hash.

### Estimation flow (SSE)

1. `thinking` event opens the stream.
2. Validate chat session is in `ready` status (server-enforced — no
   client gating).
3. Generate structured estimate via OpenAI structured outputs.
4. Emit `estimate` event with full `EstimationOutput` JSON.
5. Stream narrative tokens via plain `data:` frames (escaped `\n`).
6. Mark chat session `completed`.
7. Emit `done` event — additionally carries `routing_tags` for partner
   routing (Phase 2F).

### Configuration

Merge order (high → low precedence): env (`EFOFX_*`, `__` for nested
keys) → `config/efofx-core.toml` → struct defaults. See
`apps/efofx-core/.env.example` for required keys.

---

## 2. efofx-widget (Embeddable Chat Widget)

**Path:** `apps/efofx-widget/`
**Stack:** Vite + React 19 + TypeScript
**Status:** Repointed at `efofx-core` in Phase 3.1.

### Wire shape (post-cutover)

- Path prefix `/v1/...` (no outer `/api`).
- Auth header `x-api-key: sk_live_...`.
- Two-call chat flow: `POST /v1/chat/sessions` (create) →
  `POST /v1/chat/sessions/{id}/messages` (append).
- Plural widget paths: `/v1/widget/leads`, `/v1/widget/events`,
  `/v1/widget/consultations`.

### Embedding

```html
<script src="embed.js"
  data-api-key="sk_live_..."
  data-mode="floating"
  data-container="my-container-id">
</script>
```

Or programmatic: `efofxWidget.init({ apiKey: '...', mode: 'floating' })`.

### Components & hooks

| Module | Purpose |
|--------|---------|
| `App.tsx` | Root — fetches branding, applies CSS variables to Shadow root |
| `ShadowDOMWrapper.tsx` | Creates Shadow DOM at mount, renders React tree inside |
| `FloatingButton.tsx` | Floating widget toggle |
| `ChatPanel.tsx` | State machine: idle → chatting → lead_capture → generating → result |
| `LeadCaptureForm.tsx` | Name/email/phone form |
| `ConsultationForm.tsx` | Extended contact form |
| `ConsultationCTA.tsx` | Consultation modal trigger |
| `NarrativeStream.tsx` | Streaming narrative renderer |
| `useChat()` | Two-call create/append flow against `/v1/chat/sessions` |
| `useEstimateStream()` | SSE consumer for `:generate-estimate` |
| `useBranding()` | Public branding fetch |

---

## 3. efofx-dashboard (Tenant Dashboard)

**Path:** `apps/efofx-dashboard/`
**Stack:** Vite + React + TypeScript + TanStack Query + Recharts +
`@supabase/supabase-js`
**Status:** Repointed at `efofx-core` and migrated to Supabase auth in
Phase 3.2 / 3.3.

### Auth

- Sign-in via `supabase.auth.signInWithPassword`.
- Session persisted by the Supabase SDK; the axios interceptor pulls
  the JWT from `supabase.auth.getSession()` on every call (auto-refresh).
- Sign-out clears the session and redirects to `/login`.

### Pages
- **Login** — Supabase email/password sign-in.
- **Dashboard** — Calibration metrics:
  - ThresholdProgress (minimum outcome threshold)
  - CalibrationMetrics (accuracy stats)
  - AccuracyBucketBar (histogram)
  - AccuracyTrendLine (time-series chart via Recharts)
  - ReferenceClassTable (breakdown by class)
  - DateRangeFilter (all/1m/3m/1y)
  - Sign-out button

### What's missing (deferred to Phase 4)
- Lead list and detail views
- Tenant settings (branding, BYOK)
- API key management page
- User profile / account settings
- Reference-class CRUD

---

## 4. Shared Packages

### efofx-ui (`packages/efofx-ui/`)
React component library exported as `@efofx/ui`:
- **ChatBubble** — Single message bubble (user/assistant), plain class names for Shadow DOM
- **EstimateCard** — Displays structured EstimationOutput (costs, timeline, adjustments, assumptions)
- **ErrorBoundary** — React error boundary wrapper
- **LoadingSkeleton** — Placeholder skeleton
- **TypingIndicator** — Animated dots while waiting for LLM

### efofx-shared (`packages/efofx-shared/`)
Pure Python package consumed only by the archived FastAPI app. Retained
as a workspace member solely so the archive remains buildable for
historical reference. New work targets the Rust crates in
`apps/efofx-core/crates/`.

---

## 5. Supporting Apps

### synthetic-data-generator (`apps/synthetic-data-generator/`)
Scripts to seed MongoDB with reference class data. The Rust core also
ships `cargo run --bin seed-references` which is the authoritative
seeder going forward.

### estimator-mcp-functions (`apps/estimator-mcp-functions/`)
DigitalOcean Functions (Node.js) for reference class data via MCP
protocol. Partially implemented.

### estimator-project (`apps/estimator-project/`)
Legacy reference implementation. Kept for historical reference only.

### apps/archive/efofx-estimate-fastapi/
The original FastAPI backend, decommissioned 2026-04-28. **Not
authoritative.** See `apps/archive/README.md`.

---

## 6. CI

`.github/workflows/ci.yml` runs:
- **TypeScript Checks** — `@efofx/ui` typecheck, widget build, dashboard build.
- **Rust Checks (efofx-core)** — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo build --release`.

The previous `python-checks` job (FastAPI pytest, efofx-shared
isolation test) was removed in Phase 3.5.
