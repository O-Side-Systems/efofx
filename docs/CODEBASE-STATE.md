# Efofx Codebase State

**Last Updated:** 2026-04-15
**Purpose:** Living reference for any developer or AI agent working in this codebase. Update this document as the codebase evolves.

---

## Repository Structure

```
efofx-workspace/              # npm workspaces monorepo
  apps/
    efofx-estimate/           # FastAPI backend (Python) — the core service
    efofx-widget/             # Embeddable React chat widget
    efofx-dashboard/          # Tenant dashboard (React)
    estimator-mcp-functions/  # DigitalOcean serverless MCP functions (Node.js)
    estimator-project/        # Legacy reference implementation (unused)
    synthetic-data-generator/ # Reference class seed data scripts
  packages/
    efofx-shared/             # Shared Python package (enums, crypto)
    efofx-ui/                 # Shared React components
  docs/                       # Living documentation (source of truth)
    archive/                  # Historical docs — NOT authoritative
  scripts/                    # Utility scripts (key generation)
  STANDARDS.md                # Code quality standards
```

---

## 1. efofx-estimate (FastAPI Backend)

**Path:** `apps/efofx-estimate/`
**Stack:** FastAPI, Motor (async MongoDB), OpenAI v2 SDK, Pydantic v2
**Status:** Substantially built and functional

### Architecture

```
app/
  api/           # 5 routers: routes, auth, widget, feedback_email, feedback_form, calibration
  core/          # config.py (BaseSettings), constants.py, rate_limit.py, security.py
  db/            # mongodb.py (connection, indexes, migrations), tenant_collection.py
  middleware/    # TenantAwareCORSMiddleware
  models/        # 9 Pydantic model files (chat, estimation, tenant, widget, feedback, etc.)
  services/      # 16 service classes (see below)
  templates/     # Jinja2 email templates (feedback magic link, consultation notification)
  utils/         # crypto shims, calculation_utils, validation_utils
  main.py        # App factory with lifespan (startup: MongoDB, indexes, migrations, prompts)
config/
  prompts/       # Versioned JSON prompt files (v1.0.0-scoping, -estimation, -narrative)
tests/           # pytest (unit + integration markers)
```

### API Endpoints

| Route | Method | Auth | Description |
|-------|--------|------|-------------|
| `/api/v1/chat/send` | POST | Tenant JWT/API key | Send message, get LLM follow-up |
| `/api/v1/chat/{session_id}/generate-estimate` | POST | Tenant | SSE stream: thinking → estimate JSON → narrative tokens → done |
| `/api/v1/chat/{session_id}/history` | GET | Tenant | Full conversation history |
| `/api/v1/estimate/{session_id}` | GET | Tenant | Estimation session status |
| `/api/v1/estimate/{session_id}/upload` | POST | Tenant | Image upload (endpoint exists, no vision model wired) |
| `/api/v1/feedback/submit` | POST | Tenant | Submit outcome feedback |
| `/api/v1/feedback/summary` | GET | Tenant | Feedback aggregate |
| `/api/v1/widget/branding/{api_key_prefix}` | GET | **None** (public) | Fetch contractor branding (rate limited 30/min) |
| `/api/v1/widget/lead` | POST | API key | Save lead capture form |
| `/api/v1/widget/consultation` | POST | API key | Save consultation request + email notification |
| `/api/v1/widget/analytics` | POST/GET | API key | Record/retrieve analytics events |
| `/api/v1/calibration/*` | GET | Tenant | Calibration metrics and trends |
| `/auth/register` | POST | None | Register new tenant (rate limited 10/hr) |
| `/auth/verify` | GET | None | Email verification |
| `/auth/login` | POST | None | JWT login |
| `/auth/refresh` | POST | None | Refresh JWT |
| `/auth/profile` | GET/PATCH | Tenant | Profile management |
| `/auth/openai-key` | POST/GET | Tenant | BYOK key store/check |
| `/health` | GET | None | Health check |

### Services

| Service | File | Purpose |
|---------|------|---------|
| ChatService | `chat_service.py` | Multi-turn conversation state machine with LLM follow-ups |
| LLMService | `llm_service.py` | OpenAI v2 integration — structured outputs via `.parse()`, streaming |
| EstimationService | `estimation_service.py` | Estimation session lifecycle, generates from chat context |
| RCFEngine | `rcf_engine.py` | Reference class matching (keyword extraction, scoring, caching) |
| ReferenceService | `reference_service.py` | Reference class CRUD, modifier application |
| CalibrationService | `calibration_service.py` | Accuracy tracking, metrics aggregation |
| PromptService | `prompt_service.py` | Versioned immutable prompt registry (loaded at startup) |
| AuthService | `auth_service.py` | JWT generation, email verification, profile management |
| BYOKService | `byok_service.py` | Per-tenant Fernet key derivation (HKDF-SHA256), encrypt/decrypt OpenAI keys |
| WidgetService | `widget_service.py` | Branding fetch, lead capture, analytics recording |
| TenantService | `tenant_service.py` | Tenant CRUD |
| FeedbackService | `feedback_service.py` | Feedback submission and querying |
| FeedbackEmailService | `feedback_email_service.py` | Email notifications (fastapi-mail) |
| MagicLinkService | `magic_link_service.py` | Feedback magic link generation (Resend API) |
| ValkeyCache | `valkey_cache.py` | Redis-compatible LLM response cache (24h TTL) |

### Key Patterns

**Multi-Tenancy:** `TenantAwareCollection` wraps Motor collections and auto-injects `tenant_id` on every query. Cross-tenant data leakage is structurally impossible.

**BYOK (Bring Your Own Key):** Tenants supply their own OpenAI API key. Key is validated via `models.list()`, encrypted with per-tenant HKDF-derived Fernet key, stored as ciphertext. Decrypted only within request scope via `get_llm_service()` dependency. **No fallback to a platform key** — returns HTTP 402 when no key stored.

**Chat State Machine:** Status flow: `active` → `ready` → `completed` / `expired`. Readiness triggered by 4 populated scoping fields (project_type, size, location, timeline) or explicit trigger phrases. LLM generates context-aware follow-ups using versioned scoping prompt.

**Estimation Flow (SSE):**
1. Emit `thinking` event
2. Retrieve chat session, validate status = "ready"
3. Generate structured estimate via `llm_service.generate_estimation()` (non-streaming, `.parse()`)
4. Emit `estimate` event with JSON `EstimationOutput`
5. Stream narrative tokens via `stream_chat_completion()`
6. Mark chat session completed
7. Emit `done` event

**Prompt Management:** 3 versioned JSON files in `config/prompts/` (scoping, estimation, narrative). Immutable — new version = new file. Registry loaded at startup (fail-fast).

### Data Models

**EstimationOutput** (OpenAI structured output):
- `total_cost_p50`, `total_cost_p80` — P50/P80 cost range
- `timeline_weeks_p50`, `timeline_weeks_p80` — Timeline range
- `cost_breakdown` — List of category estimates (P50/P80 per category)
- `adjustment_factors` — Named multipliers with reasons
- `confidence_score` — 0-100
- `assumptions` — Explicit list
- `summary` — One-paragraph plain-language summary

**ScopingContext** (extracted during chat):
- `project_type`, `project_size`, `location`, `timeline`, `special_conditions`
- `is_ready()` returns true when project_type + size + location + timeline are populated

**Tenant:** company_name, email, hashed_password, hashed_api_key, tier (trial/paid), encrypted_openai_key, settings (branding, allowed_origins)

### Dependencies (from pyproject.toml)
FastAPI 0.116.1, Motor 3.3.2, Pydantic 2.11.7, OpenAI SDK, PyJWT, pwdlib[bcrypt], Valkey 6.1.0, slowapi, Resend, fastapi-mail, Jinja2

### Environment Variables
See `apps/efofx-estimate/.env.example` for the full list. Key vars:
- `MONGO_URI`, `MONGO_DB_NAME` — MongoDB Atlas connection
- `OPENAI_API_KEY`, `OPENAI_MODEL` (gpt-4o-mini default)
- `MASTER_ENCRYPTION_KEY` — Root key for BYOK encryption
- `JWT_SECRET_KEY` — Auth signing
- `VALKEY_URL` — Redis cache (graceful degradation if absent)
- `RESEND_API_KEY` — Magic link emails (optional)

---

## 2. efofx-widget (Embeddable Chat Widget)

**Path:** `apps/efofx-widget/`
**Stack:** Vite + React 19 + TypeScript
**Status:** Functional — core flow works but had bugs during demo

### Embedding

```html
<script src="embed.js"
  data-api-key="sk_live_..."
  data-mode="floating"
  data-container="my-container-id">
</script>
```

Or programmatic: `efofxWidget.init({ apiKey: '...', mode: 'floating' })`

### Components

| Component | Purpose |
|-----------|---------|
| `App.tsx` | Root — fetches branding, applies CSS variables to Shadow root |
| `ShadowDOMWrapper.tsx` | Creates Shadow DOM at mount, renders React tree inside |
| `FloatingButton.tsx` | Floating widget toggle button (bottom-right) |
| `ChatPanel.tsx` | Main orchestrator — state machine: idle → chatting → lead_capture → generating → result |
| `LeadCaptureForm.tsx` | Name/email/phone form (after chat reaches "ready") |
| `ConsultationForm.tsx` | Extended contact form with message field |
| `ConsultationCTA.tsx` | Call-to-action button/modal for consultation |
| `NarrativeStream.tsx` | Renders streaming narrative text during result phase |

### Hooks

| Hook | Purpose |
|------|---------|
| `useChat()` | Manages chat session, sends messages via `/chat/send`, tracks readiness |
| `useEstimateStream()` | Handles SSE connection for estimate + narrative streaming |
| `useBranding()` | Fetches branding from `/widget/branding/{prefix}` (public, no auth) |

### Modes
- **floating**: FloatingButton toggle, ChatPanel slides up (default)
- **inline**: ChatPanel fills container, auto-starts on mount

### Theming
Branding is fetched from the backend and applied as CSS custom properties on the Shadow root: `--brand-primary`, `--brand-secondary`, `--brand-accent`. All CSS uses these variables for brand consistency.

---

## 3. efofx-dashboard (Tenant Dashboard)

**Path:** `apps/efofx-dashboard/`
**Stack:** Vite + React + TypeScript + TanStack Query + Recharts
**Status:** Half-built — calibration metrics only, no lead management

### Current Pages
- **Login** — JWT login (stub, not fully wired)
- **Dashboard** — Calibration metrics only:
  - ThresholdProgress (minimum outcome threshold)
  - CalibrationMetrics (accuracy stats)
  - AccuracyBucketBar (histogram)
  - AccuracyTrendLine (time-series chart via Recharts)
  - ReferenceClassTable (breakdown by class)
  - DateRangeFilter (all/1m/3m/1y)

### What's Missing
- Lead list and detail views
- Tenant settings (branding configuration, BYOK key management)
- API key management page
- User profile / account settings
- Any CRUD for reference classes or feedback

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
Pure Python package (zero FastAPI/Motor dependencies, verified by isolation test):
- **Enums:** EstimationStatus, Region (8 California/Arizona/Nevada regions), ReferenceClassCategory, CostBreakdownCategory
- **Crypto:** HKDF-SHA256 key derivation, Fernet encrypt/decrypt for BYOK keys, key masking

---

## 5. Supporting Apps

### synthetic-data-generator (`apps/synthetic-data-generator/`)
Scripts to seed MongoDB with reference class data:
- 7 construction types (pools, ADUs, kitchens, bathrooms, landscaping, roofing, flooring)
- 4 California regions
- ~28 reference classes with P50/P80/P95 distributions and cost breakdowns

### estimator-mcp-functions (`apps/estimator-mcp-functions/`)
DigitalOcean Functions (Node.js) for reference class data via MCP protocol. Partially implemented — architecture defined but endpoints stubbed.

### estimator-project (`apps/estimator-project/`)
Legacy reference implementation. Redundant with efofx-estimate. Kept for historical reference only.

---

## 6. What Works vs. What Doesn't

### Fully Functional
- Multi-tenant backend with hard isolation
- BYOK key management (validate, encrypt, store, decrypt per-request)
- Chat conversation state machine with LLM follow-ups
- Estimation generation with structured outputs + SSE narrative streaming
- Widget embedding with Shadow DOM isolation and dynamic branding
- Lead capture and consultation forms
- Widget analytics (widget_view, chat_start, estimate_complete)
- Rate limiting (per-tier, per-IP for public endpoints)
- Auth system (register, verify email, login, JWT refresh)
- Feedback submission system
- Synthetic data seeding

### Partially Built / Needs Work
- Dashboard (only calibration metrics, no lead management or settings)
- Login flow (page exists, auth integration incomplete)
- Feedback email delivery (templates exist, Resend/SMTP partially configured)
- Calibration service (data model exists, aggregation pipeline incomplete)
- MCP functions (architecture defined, endpoints stubbed)

### Not Built
- Lead management dashboard (list, detail, export)
- Tenant settings UI (branding config, BYOK management)
- Marketing/demo site
- Contractor routing (post-estimate → find contractors by type)
- Chat length limits / abuse mitigation beyond rate limiting
- Platform key fallback for alpha tenants
- Image processing (upload endpoint exists, no vision model)
- Deployment automation (config files exist, not tested end-to-end)
- Monitoring/alerting (Sentry removed, no replacement)
