# Phase 1.1 — Workflow Inventory

**Source of truth:** `apps/efofx-estimate/app/api/` on `rust-port` branch, 2026-04-22.
**Purpose:** Map every FastAPI endpoint to its Rust disposition — keep / rework / drop — before drafting OpenAPI in Phase 1.2. Every downstream decision (domain types, module boundaries, OpenAPI) references this document.

**Dispositions:**
- **keep** — same contract, same behavior, new language
- **rework** — behavior survives but shape changes (e.g. path rename, body restructure)
- **supabase** — endpoint deleted; Supabase handles it
- **defer** — not needed for v1; revisit later
- **drop** — gone, no replacement

---

## 1. Path Unification Decision

**Today:** Inconsistent — `/auth/*`, `/api/v1/chat/*`, `/api/v1/widget/*`, `/api/v1/feedback/*`, `/calibration/*`, `/status`. Some routes have the `/api/v1` prefix, some don't, and "auth" lives at the root.

**Decision for Rust:** Everything externally consumed sits under `/v1/*`. Health and OpenAPI discovery stay at the root (they're platform-level, not versioned business API). Authenticated user/tenant scope uses `/v1/me/*` (REST convention — "the authenticated principal"). Public widget endpoints stay grouped under `/v1/widget/*`.

The table below uses the new Rust paths. Old FastAPI paths are given for reference in the rightmost column.

---

## 2. Endpoint Dispositions

### 2.1 Identity & Tenant (Phase 2A)

| Rust path | Method | Auth | Disposition | Rust module | FastAPI source |
|-----------|--------|------|-------------|-------------|----------------|
| *(n/a)* | POST | — | **supabase** | — | `POST /auth/register` |
| *(n/a)* | GET  | — | **supabase** | — | `GET /auth/verify` |
| *(n/a)* | POST | — | **supabase** | — | `POST /auth/login` |
| *(n/a)* | POST | — | **supabase** | — | `POST /auth/refresh` |
| `/v1/me` | GET  | Supabase JWT OR API key | **rework** | `identity::me` | `GET /auth/profile` |
| `/v1/me` | PATCH | Supabase JWT OR API key | **rework** | `identity::me` | `PATCH /auth/profile` |
| `/v1/me/openai-key` | PUT | Supabase JWT | **rework** | `identity::byok` | `PUT /auth/openai-key` |
| `/v1/me/openai-key/status` | GET | Supabase JWT OR API key | **rework** | `identity::byok` | `GET /auth/openai-key/status` |
| `/v1/me/openai-key` | DELETE | Supabase JWT | **new** | `identity::byok` | — (not in FastAPI; add for key removal) |
| `/v1/me/api-keys:rotate` | POST | Supabase JWT | **new** | `identity::api_keys` | — (key rotation has no Supabase equivalent; widget keys are our mechanism) |

**Notes:**
- Four endpoints (`/auth/register`, `/auth/verify`, `/auth/login`, `/auth/refresh`) are deleted outright — Supabase owns registration, email verify, login, and refresh. Dashboard calls Supabase JS SDK directly for these.
- `/auth/profile` becomes `/v1/me` so the shape reads correctly: it's the authenticated principal's profile, merged from Supabase user claims + local tenant state.
- PATCH `/v1/me` cannot modify email or password (those live in Supabase). It only touches tenant-owned settings: `company_name`, `settings.branding`, `settings.allowed_origins`, `settings.routing_config`.
- `DELETE /v1/me/openai-key` is new — FastAPI has no way to remove a stored BYOK key short of overwriting it. We should support explicit removal.
- `POST /v1/me/api-keys:rotate` is new — widget API keys live only in our system; users need a way to rotate them. Issues a new plaintext key exactly once, invalidates the old.

### 2.2 Chat & Session State (Phase 2B)

| Rust path | Method | Auth | Disposition | Rust module | FastAPI source |
|-----------|--------|------|-------------|-------------|----------------|
| `/v1/chat/sessions` | POST | API key OR Supabase JWT | **rework** | `chat::sessions` | *(implicit: `/chat/send` with no session_id created one)* |
| `/v1/chat/sessions/{id}/messages` | POST | API key OR Supabase JWT | **rework** | `chat::messages` | `POST /api/v1/chat/send` |
| `/v1/chat/sessions/{id}` | GET | API key OR Supabase JWT | **rework** | `chat::sessions` | `GET /api/v1/chat/{session_id}/history` |
| `/v1/chat/sessions/{id}:generate-estimate` | POST | API key OR Supabase JWT | **keep** (SSE) | `estimation::stream` | `POST /api/v1/chat/{session_id}/generate-estimate` |

**Notes:**
- FastAPI overloaded `POST /chat/send` to both create sessions (when `session_id` omitted) and append messages. Splitting into explicit `POST /v1/chat/sessions` (create) and `POST /v1/chat/sessions/{id}/messages` (append) makes state transitions readable and OpenAPI-friendly.
- Generate-estimate uses the Google AIP-136 custom-verb convention (`:generate-estimate`) to signal a non-CRUD action on a specific session. Keeps resource semantics honest.
- Renaming `/history` → the plain resource GET (`/v1/chat/sessions/{id}`) is cleaner: "get the session" naturally includes its messages.

### 2.3 Estimation & Reference Classes (Phase 2C)

| Rust path | Method | Auth | Disposition | Rust module | FastAPI source |
|-----------|--------|------|-------------|-------------|----------------|
| `/v1/estimates/{session_id}` | GET | API key OR Supabase JWT | **keep** | `estimation::sessions` | `GET /api/v1/estimate/{session_id}` |
| `/v1/estimates/{session_id}/images` | POST | API key OR Supabase JWT | **defer** | — | `POST /api/v1/estimate/{session_id}/upload` |

**Notes:**
- The image upload endpoint is a **stub** — there's no vision-model integration. Do not port it in v1. If multimodal intake becomes a real feature, revisit as a separate phase.
- The estimation generation itself is triggered through `/v1/chat/sessions/{id}:generate-estimate` (above), not as a separate POST. That's faithful to how FastAPI works today — estimation is an action on a chat session, not an independent resource creation.

### 2.4 Widget Public & Authenticated (Phase 2D)

| Rust path | Method | Auth | Disposition | Rust module | FastAPI source |
|-----------|--------|------|-------------|-------------|----------------|
| `/v1/widget/branding/{api_key_prefix}` | GET | **public** (rate-limited) | **keep** | `widget::public` | `GET /api/v1/widget/branding/{api_key_prefix}` |
| `/v1/widget/leads` | POST | API key | **rework** | `widget::leads` | `POST /api/v1/widget/lead` (singular → plural) |
| `/v1/widget/consultations` | POST | API key | **rework** | `widget::consultations` | `POST /api/v1/widget/consultation` (singular → plural) |
| `/v1/widget/events` | POST | API key | **rework** | `widget::analytics` | `POST /api/v1/widget/analytics` |
| `/v1/widget/events` | GET | Supabase JWT | **rework** | `widget::analytics` | `GET /api/v1/widget/analytics` |

**Notes:**
- Branding endpoint stays **public** — the widget fetches this before any auth context exists. Rate limit (30/min/IP) stays as first-class policy.
- Singular-to-plural rename for leads/consultations aligns with REST resource conventions (it's a collection of leads, so `POST /leads`).
- `analytics` → `events` is a small rename to better describe what the endpoint does (emit/query events, not generate analytics). Flexible; open to pushback.
- Analytics READ split from WRITE by auth: writes are machine (API key, widget embed), reads are human (JWT, dashboard).

### 2.5 Leads & Consultations (Phase 2D/2E crossover)

Covered above under Widget. Leads are captured via widget (public-facing), read via dashboard (JWT).

**To add in Phase 2D/2E (not in FastAPI):**

| Rust path | Method | Auth | Disposition | Rust module |
|-----------|--------|------|-------------|-------------|
| `/v1/leads` | GET (list) | Supabase JWT | **new** | `leads::dashboard` |
| `/v1/leads/{id}` | GET | Supabase JWT | **new** | `leads::dashboard` |
| `/v1/leads/{id}` | PATCH (status) | Supabase JWT | **new** | `leads::dashboard` |

These are required for the dashboard (PRD §6.5, IMPLEMENTATION-PLAN Phase 2). FastAPI never exposed a dashboard read path for leads — they were written by widget, viewed only in the DB. Adding them here is part of closing the dashboard gap.

### 2.6 Feedback (Phase 2E)

| Rust path | Method | Auth | Disposition | Rust module | FastAPI source |
|-----------|--------|------|-------------|-------------|----------------|
| `/v1/feedback` | POST | API key OR Supabase JWT | **keep** | `feedback::submit` | `POST /api/v1/feedback/submit` |
| `/v1/feedback/summary` | GET | Supabase JWT | **keep** | `feedback::summary` | `GET /api/v1/feedback/summary` |
| `/v1/feedback/email-requests` | POST | Supabase JWT | **rework** | `feedback::magic_link` | `POST /api/v1/feedback/request-email/{session_id}` (session_id moves to body) |
| `/v1/feedback/forms/{token}` | GET | **email magic-link token** | **keep** (HTML render) | `feedback::form` | `GET /api/v1/feedback/form/{token}` |
| `/v1/feedback/forms/{token}` | POST | **email magic-link token** | **keep** (form submit) | `feedback::form` | `POST /api/v1/feedback/form/{token}` |

**Notes:**
- Moving `session_id` from URL to body on the email-request endpoint: it's an input, not a resource identifier. The resource is "feedback email request."
- Magic-link form endpoints return HTML (Jinja2 today, Askama or minijinja in Rust). OpenAPI marks these as `text/html` responses rather than JSON — an exception to the envelope rule, and that's fine.
- Idempotency of `GET /v1/feedback/forms/{token}` is non-negotiable (email scanners pre-fetch the link). Preserve the "mark opened, don't consume" pattern exactly.

### 2.7 Calibration (Phase 2E)

| Rust path | Method | Auth | Disposition | Rust module | FastAPI source |
|-----------|--------|------|-------------|-------------|----------------|
| `/v1/calibration/metrics` | GET | Supabase JWT | **keep** | `calibration::metrics` | `GET /api/v1/calibration/metrics` |
| `/v1/calibration/trend` | GET | Supabase JWT | **keep** | `calibration::trend` | `GET /api/v1/calibration/trend` |

**Notes:**
- Preserve the 10-outcome threshold behavior (CALB-03): if fewer than 10 real outcomes, response is `{ below_threshold: true, count }` instead of metrics. That's a deliberate product decision (no cherry-picked numbers). Document in OpenAPI as a oneOf response.

### 2.8 Partner Routing (Phase 2F)

**New in Rust — no FastAPI equivalent:**

| Rust path | Method | Auth | Disposition | Rust module |
|-----------|--------|------|-------------|-------------|
| `/v1/integration/contractor-match` | POST | API key (partner) | **new** | `integration::routing` |

Included here so Phase 1.2 OpenAPI has a slot for it. Definition lands in Phase 2F.

### 2.9 Admin / Platform (Phase 2 last, or deferred)

| Rust path | Method | Auth | Disposition | Rust module | FastAPI source |
|-----------|--------|------|-------------|-------------|----------------|
| *(n/a)* | GET | — | **defer** (no auth today) | — | `GET /admin/tenants` |
| *(n/a)* | GET | — | **defer** (no auth today) | — | `GET /admin/reference-classes` |

The FastAPI admin endpoints have **no auth guard** in the current code — they're half-built. Do not port as-is. If/when we need an admin surface, design it deliberately (Supabase role claim + explicit middleware + a separate route prefix, e.g. `/v1/admin/*`).

### 2.10 Platform / System

| Rust path | Method | Auth | Disposition | Notes |
|-----------|--------|------|-------------|-------|
| `/health` | GET | public | **keep** (already in Phase 0) | Mongo-reachability probe |
| `/openapi.json` | GET | public | **keep** (already in Phase 0) | Contract discovery |
| *(n/a)* | GET | — | **drop** | `GET /status` (duplicates `/health`) |

---

## 3. Pydantic Models → Rust Domain Types

Source for Phase 1.4. Every model below has a Rust equivalent in `efofx-domain` or a module-local request/response struct (utoipa derives on everything externally visible).

**Already done in Phase 0:** `TenantId`, `SessionId`, `Tenant`, `TenantTier`.

**To add in Phase 1.4 (in `efofx-domain`):**

| Rust type | Source Pydantic class | Lives in | Notes |
|-----------|-----------------------|----------|-------|
| `ChatStatus` | str on `ChatSession.status` | `efofx-domain::chat` | enum: `Active`, `Ready`, `Completed`, `Expired` |
| `ChatMessage` | `ChatMessage` | `efofx-domain::chat` | + `MessageRole` enum: `User`, `Assistant`, `System` |
| `ChatSession` | `ChatSession` | `efofx-domain::chat` | `TenantId`, `SessionId`, `ChatStatus`, Vec<ChatMessage>, `ScopingContext`, timestamps |
| `ScopingContext` | `ScopingContext` | `efofx-domain::scoping` | 5 optional fields + `is_ready()` method returning bool |
| `EstimationStatus` | enum from `efofx_shared` | `efofx-domain::estimation` | 5-variant enum (see §4.1) |
| `EstimationOutput` | `EstimationOutput` | `efofx-domain::estimation` | Full structured estimate shape |
| `CostCategoryEstimate` | `CostCategoryEstimate` | `efofx-domain::estimation` | per-category breakdown |
| `CostBreakdownCategory` | enum from `efofx_shared` | `efofx-domain::estimation` | 7 variants (materials, labor, …) |
| `AdjustmentFactor` | `AdjustmentFactor` | `efofx-domain::estimation` | name, multiplier, reason |
| `Region` | enum from `efofx_shared` | `efofx-domain::geo` | 8 variants (SoCal - Coastal, etc.) |
| `ReferenceClass` | `ReferenceClass` | `efofx-domain::reference` | category, subcategory, keywords, distributions |
| `ReferenceClassCategory` | enum from `efofx_shared` | `efofx-domain::reference` | 7 variants |
| `Lead` | `LeadCaptureRequest` + status | `efofx-domain::leads` | + `LeadStatus` enum: `New`, `Contacted`, `Converted`, `Closed` |
| `ConsultationRequest` | `ConsultationRequest` | `efofx-domain::leads` | extends lead with `message` field |
| `BrandingConfig` | `BrandingConfigResponse` | `efofx-domain::branding` | colors, logo, text, company_name, locale |
| `FeedbackDocument` | `FeedbackDocument` | `efofx-domain::feedback` | + `DiscrepancyReason` enum (6 variants) |
| `AnalyticsEventType` | validated set | `efofx-domain::analytics` | enum: `WidgetView`, `ChatStart`, `EstimateComplete` |

Request/response DTOs (e.g. `ChatRequest`, `RegisterRequest`) live next to their handlers in the server crate, not in `efofx-domain`. `efofx-domain` is domain values; DTOs are transport shapes.

---

## 4. Enum & Magic-String Classification Preview (Phase 1.3)

Full constants classification ships in `constants-classification.md`. This section surfaces the inputs.

### 4.1 Typed enums (compile-time)

Confirmed values from `packages/efofx-shared/efofx_shared/core/constants.py`:

- **`EstimationStatus`**: `Initiated`, `InProgress`, `Completed`, `Cancelled`, `Expired`
- **`ReferenceClassCategory`**: `Residential`, `Commercial`, `Industrial`, `Infrastructure`, `Landscaping`, `Renovation`, `NewConstruction`
- **`CostBreakdownCategory`**: `Materials`, `Labor`, `Equipment`, `Permits`, `Design`, `Contingency`, `ProfitMargin`
- **`Region`**: 8 Southwest US regions (see source)

Local to `efofx-estimate/app/core/constants.py`:

- **`DB_COLLECTIONS`**: 11 collection names — stay as compile-time constants in the storage layer, not config (collection names don't vary per tenant).
- **`HTTP_STATUS`**: delete; use `http::StatusCode` in Rust.
- **`API_MESSAGES`**: becomes part of the error-code registry in `efofx-openapi::envelope` as typed error codes. Not config.

### 4.2 Runtime config (tenant- or env-configurable)

- `ESTIMATION_CONFIG`: `MAX_CHAT_MESSAGES`, `MAX_PROJECT_DESCRIPTION_LENGTH`, `MIN_PROJECT_DESCRIPTION_LENGTH`, `DEFAULT_CONFIDENCE_THRESHOLD`, `MAX_REFERENCE_PROJECTS`, `MIN_REFERENCE_PROJECTS` → all move to `AppConfig.estimation.*` in `efofx-config` with existing values as defaults.
- Rate limits: tier-based limits (`trial: 20/minute`, `paid: 100/minute`) and endpoint-specific limits (register `10/hour`, login `5/15min`, refresh `30/min`, branding `30/min`, analytics GET `10/min`) all become entries in `AppConfig.rate_limits.*` — decomposed into a flat key-per-bucket schema.
- `FILE_UPLOAD_CONFIG` → defers with the image endpoint.

### 4.3 Documented defaults (rarely changed)

- JWT expiries (access 20min, refresh 14d): become `auth.jwt.access_ttl_seconds`, `auth.jwt.refresh_ttl_seconds` — but those are Supabase-owned now. Our service only *verifies* JWTs; we don't issue them.
- Widget analytics event set (`{widget_view, chat_start, estimate_complete}`): compile-time enum (§4.1), not config. If partners need custom events later, revisit.
- Magic-link TTL (72h), verification-token TTL: become `feedback.magic_link_ttl_hours` (default 72). Verification tokens are Supabase's problem.

### 4.4 SSE event types (compile-time enum)

Currently string literals in `chat_service.py` / handlers:

- `thinking`, `estimate`, `done`, `error`, plus the raw `data:` narrative tokens.

Becomes `enum SseEvent { Thinking, Estimate(EstimationOutput), NarrativeToken(String), Done, Error(ApiError) }` in `efofx-domain::sse`. The serialized `event:` header comes from a `fn kind() -> &'static str` method on the enum.

---

## 5. Cross-Cutting Observations

1. **Auth unification possible.** Today, `/auth/*` and `/api/v1/*` use different auth paths (`Depends(get_current_tenant)` vs raw decorators). In Rust both live behind the same middleware stack; only the `/v1/widget/branding/*` endpoint is public. This is a simplification win.

2. **Two auth surfaces, one `TenantContext`.** Supabase JWT and widget API key must both mint the same `TenantContext` (Phase 0.4 already enforces this structurally). Handlers never branch on which credential was used.

3. **Email adapter is a leak.** FastAPI uses `fastapi-mail` for feedback notifications AND `resend` for magic-link emails. In Rust, one `EmailProvider` trait with a `ResendProvider` impl. Keep it trait-shaped so a dev-mode `LoggingProvider` stub works for local dev without an API key.

4. **Prompt storage stays file-based.** `config/prompts/*.json` loaded at startup. No change — the pattern works and aligns with TADR §9.2. Rust: read once in `main()`, hand an `Arc<PromptRegistry>` into the chat and estimation services.

5. **Valkey/Redis cache is optional today.** `valkey_cache.py` gracefully degrades when unreachable. Preserve this — the Rust `Cache` trait gets a `NullCache` impl for dev. Only rate limiting should require a real cache in production (to share limits across instances).

6. **Two-tier Mongo access.** Today, `TenantAwareCollection` auto-injects `tenant_id`. In Rust, repositories take `&TenantContext` and build that filter explicitly. Less magic, same guarantee, compile-time enforced.

---

## 6. Out of Scope (Explicit Non-Goals for the Port)

- Image upload / vision intake (no real implementation exists)
- Admin endpoints (half-built, no auth)
- Multi-LLM provider fallback (single OpenAI provider, per TADR §9.3)
- Queue-based background jobs (TADR §11.3 — not yet)
- MCP function endpoints (`apps/estimator-mcp-functions` is architectural-only today)

These are tracked in `docs/IMPLEMENTATION-PLAN.md` "Technical Debt to Track" and stay there.

---

## 7. Decisions Resolved (2026-04-22)

All design calls below were confirmed with Brett before writing Phase 1.2. Path/shape conventions recorded here bind Phase 1.2 (OpenAPI), Phase 1.4 (domain types), and every handler from Phase 2 onward.

1. **Path scheme:** All external routes under `/v1/*`. `/health` and `/openapi.json` stay at the root (platform-level, unversioned).
2. **Authenticated self:** `/v1/me/*` — REST convention for the authenticated principal. Not `/v1/tenants/current` or `/v1/profile`.
3. **Actions on resources:** Google AIP-136 custom verbs (`POST /v1/chat/sessions/{id}:generate-estimate`). The `:verb` suffix signals "this is an action, not a sub-resource" to both readers and OpenAPI tooling.
4. **Widget path renames:** Plural resource nouns and clearer naming (`/v1/widget/leads`, `/v1/widget/consultations`, `/v1/widget/events`). Widget client code updates in Phase 3; consistency ranks above client churn.
5. **Dashboard endpoints for leads:** Land in Phase 2D alongside widget writes, so the dashboard has server support waiting at Phase 3 cutover. No phase interleaving.
6. **Image upload:** Deferred. Not in v1. Revisit if multimodal intake becomes a real product requirement.

---

## 8. Scoping-extractor divergences (2026-04-22, Phase 2B.4)

The Rust `scoping::extract_scoping` port intentionally differs from FastAPI's `_update_scoping_context` in one place:

- **Location keyword ordering.** FastAPI's dict ordered the 2-letter code `la` before the multi-word alias `inland empire`. Because matching is case-insensitive substring, `"Inland Empire home"` hit `la` (inside `in**la**nd`) first and routed to *SoCal - Coastal* instead of *SoCal - Inland*. The Rust port moves both 2-letter codes (`la`, `sf`) to the end of the list so every multi-word regional alias wins first. This changes behavior for a small set of messages containing those substrings; no other callers are exercising the region tag in production yet.

Every other quirk of the FastAPI extractor is preserved for parity, including:

- `renovation`/`remodel` matching before `kitchen`/`bathroom`, so `"Kitchen remodel"` maps to `renovation` rather than `kitchen renovation`.
- The timeline pattern list having seasons before the `by end of …` phrase, so `"by end of summer"` captures only `summer`.
- Confirmation word matching being whole-token (no punctuation stripping), so `"Yes!"` does not match `yes`.
