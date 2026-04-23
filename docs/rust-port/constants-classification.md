# Phase 1.3 — Constants Classification

**Sources walked:**
- `packages/efofx-shared/efofx_shared/core/constants.py`
- `apps/efofx-estimate/app/core/constants.py`
- `apps/efofx-estimate/app/core/config.py`
- `apps/efofx-estimate/app/core/rate_limit.py`
- Scattered literals in `apps/efofx-estimate/app/services/*.py` and `app/api/*.py`

**Classification rule (from TADR §3.1):** Every value goes into one of four buckets.

| Bucket | Rust home | When to use |
|--------|-----------|-------------|
| **Enum** | `efofx-domain` | Intrinsic, stable, type-safe discriminators (status values, roles, categories) |
| **Config** | `efofx-config::AppConfig` | Varies by environment, tenant, tier, or deployment |
| **Default** | `efofx-config::AppConfig` with documented default | Rarely changed; override only if pushed |
| **Drop** | — | Superseded by Supabase, ecosystem primitives, or removed features |

---

## 1. Enums (compile-time, in `efofx-domain`)

### 1.1 Already shipped (Phase 0)

- `TenantTier` — `Trial`, `Alpha`, `Paid` (note: Alpha is new vs FastAPI; TADR §9.3 platform-key fallback tier)

### 1.2 Pulled from `efofx_shared/core/constants.py`

Exact values confirmed against source:

| Enum | Variants | Notes |
|------|----------|-------|
| `EstimationStatus` | `Initiated`, `InProgress`, `Completed`, `Cancelled`, `Expired` | Session-level status |
| `ReferenceClassCategory` | `Residential`, `Commercial`, `Industrial`, `Infrastructure`, `Landscaping`, `Renovation`, `NewConstruction` | Reference-class top-level taxonomy |
| `CostBreakdownCategory` | `Materials`, `Labor`, `Equipment`, `Permits`, `Design`, `Contingency`, `ProfitMargin` | Drives cost decomposition output |
| `Region` | `SoCalCoastal`, `SoCalInland`, `NorCalBayArea`, `NorCalCentral`, `ArizonaPhoenix`, `ArizonaTucson`, `NevadaLasVegas`, `NevadaReno` | 8 Southwest US regions; serde-renamed to their display strings (`"SoCal - Coastal"`, etc.) |

### 1.3 Derived from string literals in service code

| Enum | Variants | Evidence |
|------|----------|----------|
| `ChatStatus` | `Active`, `Ready`, `Completed`, `Expired` | `services/chat_service.py` uses all four as string literals |
| `MessageRole` | `User`, `Assistant`, `System` | `models/chat.py` ChatMessage.role |
| `AnalyticsEventType` | `WidgetView`, `ChatStart`, `EstimateComplete` | `services/widget_service.py` VALID_EVENT_TYPES set |
| `DiscrepancyReason` | `ScopeChanged`, `UnforeseenIssues`, `TimelinePressure`, `VendorMaterialCosts`, `ClientChanges`, `EstimateWasAccurate` | `models/feedback.py` DiscrepancyReason enum |
| `LeadStatus` | `New`, `Contacted`, `Converted`, `Closed` | **New in Rust** — dashboard needs a status field; FastAPI never had one |
| `MagicLinkState` | `Valid`, `Expired`, `Used`, `NotFound` | `services/magic_link_service.py` return values |
| `SseEvent` | `Thinking`, `Estimate(EstimationOutput)`, `NarrativeToken(String)`, `Done`, `Error(ApiError)` | SSE frame types in estimation stream |

### 1.4 Ambient enum `AdjustmentFactorName`

Optional. FastAPI uses arbitrary strings (e.g. `"regional_modifier"`, `"access_difficulty"`). Can stay as `String` for v1 — the product differentiator is the multiplier + reason, not the name's type-safety. Revisit if the frontend wants to localize or icon-ify these.

---

## 2. Compile-time constants (not enums, in `efofx-storage` or domain crates)

| Symbol | Value | Rust home | Notes |
|--------|-------|-----------|-------|
| `DB_COLLECTIONS["TENANTS"]` | `"tenants"` | `efofx-storage::collections::TENANTS` | Collection names are intrinsic to the schema; not per-tenant or per-env |
| `DB_COLLECTIONS["CHAT_SESSIONS"]` | `"chat_sessions"` | `efofx-storage::collections::CHAT_SESSIONS` | |
| `DB_COLLECTIONS["ESTIMATES"]` | `"estimates"` | `efofx-storage::collections::ESTIMATES` | |
| `DB_COLLECTIONS["FEEDBACK"]` | `"feedback"` | `efofx-storage::collections::FEEDBACK` | |
| `DB_COLLECTIONS["FEEDBACK_TOKENS"]` | `"feedback_tokens"` | `efofx-storage::collections::FEEDBACK_TOKENS` | |
| `DB_COLLECTIONS["REFERENCE_CLASSES"]` | `"reference_classes"` | `efofx-storage::collections::REFERENCE_CLASSES` | |
| `DB_COLLECTIONS["WIDGET_LEADS"]` | `"widget_leads"` | `efofx-storage::collections::WIDGET_LEADS` | |
| `DB_COLLECTIONS["WIDGET_ANALYTICS"]` | `"widget_analytics"` | `efofx-storage::collections::WIDGET_ANALYTICS` | |
| `DB_COLLECTIONS["REFERENCE_PROJECTS"]` | `"reference_projects"` | keep only if used | `REFERENCE_PROJECTS` is listed in constants but I did not find live references — audit in Phase 2C |
| `DB_COLLECTIONS["VERIFICATION_TOKENS"]` | — | **drop** | Supabase handles email verification |
| `DB_COLLECTIONS["REFRESH_TOKENS"]` | — | **drop** | Supabase handles refresh-token storage |

**Pattern in Rust:**

```rust
// efofx-storage::collections
pub const TENANTS: &str = "tenants";
pub const CHAT_SESSIONS: &str = "chat_sessions";
// ...
```

Repositories reference these by symbol, never by string literal.

---

## 3. Runtime config (in `efofx-config::AppConfig`)

Laying out the target `AppConfig` schema. Tree shape follows TOML section conventions.

### 3.1 Currently in Phase 0 (keep as-is)

- `server.host`, `server.port`
- `mongo.uri`, `mongo.db_name`
- `supabase.url`, `supabase.issuer`, `supabase.audience`, `supabase.jwks_refresh_seconds`
- `crypto.master_key`
- `llm.provider`, `llm.model`, `llm.request_timeout_ms`, `llm.streaming_enabled`
- `features.contractor_routing`, `features.platform_key_fallback`

### 3.2 To add in Phase 1 / 2

**Cache layer:**
- `cache.url` — Valkey/Redis URI (default `"redis://localhost:6379"`)
- `cache.ttl_seconds` — LLM response cache TTL (default 86400 = 24h)
- `cache.enabled` — bool (default true); set false in local dev

**LLM tuning:**
- `llm.max_tokens` — default 4000, matches `OPENAI_MAX_TOKENS`
- `llm.temperature` — default 0.7, matches `OPENAI_TEMPERATURE`
- `llm.max_messages_per_session` — default 50 (from `ESTIMATION_CONFIG.MAX_CHAT_MESSAGES`)
- `llm.per_session_token_budget` — default None (disabled); TADR §9.4
- `llm.per_tenant_token_budget` — default None; platform-fallback tiers only
- `llm.platform_openai_key` — Option<String>; used when `features.platform_key_fallback=true` and tenant tier is `Alpha`

**Estimation input rules:**
- `estimation.max_project_description_length` — default 2000
- `estimation.min_project_description_length` — default 10
- `estimation.default_confidence_threshold` — default 0.7
- `estimation.max_reference_projects` — default 10
- `estimation.min_reference_projects` — default 3

**Rate limits** (keys follow `{bucket}.{period}` convention):
- `rate_limits.tiers.trial` — default `"20/minute"`
- `rate_limits.tiers.paid` — default `"100/minute"`
- `rate_limits.tiers.alpha` — default `"50/minute"` (new — between trial and paid)
- `rate_limits.endpoints.widget_branding` — default `"30/minute"` (per IP, public)
- `rate_limits.endpoints.widget_analytics_read` — default `"10/minute"`
- `rate_limits.storage` — enum `memory` | `valkey` (default `memory` in dev, `valkey` in prod)

Auth-register / login / refresh limits are **dropped** — Supabase owns those.

**Retention / TTL:**
- `retention.chat_session_hours` — default 24 (matches current behavior)
- `retention.magic_link_hours` — default 72 (MAGIC_LINK_TTL_HOURS)
- `retention.session_timeout_minutes` — default 30 (SESSION_TIMEOUT_MINUTES)
- `retention.lead_days` — default None (never delete); revisit per data-policy decisions
- `retention.analytics_days` — default None

**Calibration:**
- `calibration.minimum_outcomes` — default 10 (CALB-03 threshold)
- `calibration.trend_months_default` — default 12
- `calibration.trend_months_max` — default 36

**Email (Resend):**
- `email.provider` — enum `resend` | `logging` (default `logging` in dev — prints magic links to stdout)
- `email.resend_api_key` — Option<String>; required when provider = `resend`
- `email.from_address` — default `"noreply@efofx.ai"`

**Widget branding fallback:**
- `widget.default_branding.primary_color` — default `"#0b63d8"` (pick a neutral, document in code)
- `widget.default_branding.secondary_color`
- `widget.default_branding.accent_color`
- `widget.default_branding.welcome_message` — default `"How can we help with your project?"`
- `widget.default_branding.button_text` — default `"Get Estimate"`
- `widget.allowed_origins_default` — default `["*"]` (dev); prod should override per-tenant

**App base URL (for email links):**
- `app.base_url` — default `"http://localhost:8080"`; prod config must override

**Logging / observability:**
- `log.level` — default `"info"` (driven via `RUST_LOG` env var, not config)
- `log.format` — tracing-subscriber JSON; compile-time default, not config

### 3.3 To drop from config entirely

| FastAPI config key | Why dropped |
|--------------------|-------------|
| `SECRET_KEY`, `JWT_SECRET_KEY`, `JWT_ALGORITHM`, `JWT_EXPIRATION_HOURS` | Supabase issues and signs JWTs |
| `ENCRYPTION_KEY` | Duplicate of `MASTER_ENCRYPTION_KEY`; kept the latter as `crypto.master_key` |
| `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_SERVER`, `SMTP_PORT`, `SMTP_FROM`, `MAIL_*` | Email verification is Supabase's job; only feedback magic-link email remains and it goes through Resend |
| `SENTRY_DSN`, `SENTRY_ENVIRONMENT`, `SENTRY_TRACES_SAMPLE_RATE` | Sentry isn't wired up; re-add under `observability.error_tracker` if/when we pick a tool |
| `ALLOWED_HOSTS` | Middleware concern; CORS allowed-origins come from per-tenant `settings.allowed_origins`, not global |
| `ALLOWED_ORIGINS` (global) | Same as above — per-tenant via settings |
| `RATE_LIMIT_PER_MINUTE` (global) | Replaced by structured `rate_limits.*` tree |
| `APP_NAME`, `VERSION`, `DEBUG`, `ENVIRONMENT` | Compile-time constants or CI-injected build info, not runtime config |
| `MAX_ESTIMATION_SESSIONS` | Appears unused in code (grep finds only the declaration); confirm and drop |

### 3.4 Deferred (with image upload endpoint)

- `FILE_UPLOAD_CONFIG.MAX_SIZE_MB`, `ALLOWED_EXTENSIONS`, `UPLOAD_DIR`, `TEMP_DIR`

---

## 4. Documented defaults (not config, but tweakable)

These are numbers in Rust code with a comment explaining the choice. Pull into config if the need becomes real.

| Value | Location | Default | Rationale |
|-------|----------|---------|-----------|
| Calibration date-range options | `calibration::query` | `["6months", "1year", "all"]` | Matches current dashboard options; small set, unlikely to vary |
| Pagination defaults | `common::pagination` | `limit=20`, `offset=0` | Standard REST default |
| Pagination max | `common::pagination` | `limit=100` | Cap per page; guards against `GET /leads?limit=1000000` |
| SSE keep-alive interval | `sse::mod` | 15 seconds | Prevents proxies from closing idle streams |
| JWKS fetch timeout | `identity::jwks` | 5 seconds | Startup + periodic refresh |
| HTTP client request timeout | `http::client` | 30 seconds | Inherits from `llm.request_timeout_ms` for LLM calls |

---

## 5. API error code registry

Error codes live in `efofx-openapi::errors` as an enum with stable string rendering. They're neither enum-over-business-state nor config — they're an API contract.

| Code | HTTP | Message (default) |
|------|------|-------------------|
| `auth.missing_token` | 401 | "Authentication required." |
| `auth.invalid_token` | 401 | "Authentication token is invalid." |
| `auth.expired_token` | 401 | "Authentication token has expired." |
| `auth.wrong_audience` | 401 | "Authentication token not issued for this service." |
| `auth.api_key_invalid` | 401 | "Widget API key is invalid." |
| `auth.forbidden` | 403 | "Not authorized." |
| `rate_limit.exceeded` | 429 | "Too many requests. Try again later." |
| `validation.failed` | 422 | "Request validation failed." |
| `chat.session_not_found` | 404 | "Chat session not found." |
| `chat.session_expired` | 410 | "Chat session has expired." |
| `chat.session_not_ready` | 409 | "Chat session is not ready for estimation." |
| `chat.message_limit_exceeded` | 429 | "Message limit for this session reached." |
| `estimation.session_not_found` | 404 | "Estimation session not found." |
| `estimation.llm_invalid_key` | 402 | "OpenAI API key missing or invalid. Update it in Settings." |
| `estimation.llm_quota_exhausted` | 402 | "OpenAI quota exhausted. Recharge your OpenAI account." |
| `estimation.llm_transient` | 503 | "We're having trouble generating a response. Please try again." |
| `feedback.token_expired` | 410 | "This feedback link has expired." |
| `feedback.token_used` | 410 | "This feedback has already been submitted." |
| `widget.branding_not_found` | 404 | "Widget branding not found for the supplied API key prefix." |
| `widget.invalid_event_type` | 400 | "Analytics event type is not recognized." |

Messages are overridable per-tenant later (localization) but not per-deployment. Codes are immutable once published — treat like public API.

---

## 6. Summary Count

| Bucket | Count |
|--------|-------|
| New Rust enums (pending in `efofx-domain`) | **12** (ChatStatus, MessageRole, AnalyticsEventType, DiscrepancyReason, LeadStatus, MagicLinkState, SseEvent) + 4 ported from `efofx_shared` + TenantTier (done) |
| Compile-time DB constants | **9** kept, 2 dropped |
| Config keys to add in Phase 1/2 | **~40** |
| Config keys dropped | **~15** |
| Documented defaults | **6** |
| API error codes | **20** (starting registry) |

---

## 7. Validation Checklist for Phase 1.4

When flesh­ing out `efofx-domain` (Phase 1.4), confirm each enum:
- [ ] derives `Debug`, `Clone`, `Copy` (where small), `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`, `ToSchema` (utoipa)
- [ ] uses `#[serde(rename_all = "snake_case")]` or explicit per-variant rename to match FastAPI wire format byte-for-byte
- [ ] for `Region`, uses explicit `#[serde(rename = "...")]` on each variant because the display strings have hyphens and spaces

When adding config keys (Phase 1/2), confirm each:
- [ ] has a default (or is documented as required at startup)
- [ ] has validation (e.g. `audience` non-empty, `jwks_refresh_seconds > 0`)
- [ ] is referenced by its typed accessor, not a string key lookup
- [ ] has a `.env.example` entry showing the env-var form (`EFOFX_<SECTION>__<KEY>`)
