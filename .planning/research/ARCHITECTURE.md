# Architecture Research

**Domain:** Multi-tenant SaaS estimation platform — v1.1 Feature Integration
**Researched:** 2026-02-28
**Confidence:** HIGH (existing codebase reviewed directly; all integration patterns verified against actual source)

---

## Context: Brownfield Integration for v1.1

v1.0 is complete and deployed. This document covers only what changes or is new in v1.1. The baseline architecture (FastAPI layered service pattern, TenantAwareCollection isolation, JWT + API key dual auth, Shadow DOM widget) is stable and must not be disturbed.

**What is complete and working in v1.0:**
- `app/api/` — three routers: `api_router`, `auth_router`, `widget_router`
- `app/services/` — all core services implemented: `FeedbackService`, `LLMService`, `EstimationService`, `ChatService`, `AuthService`, `BYOKService`, `WidgetService`
- `app/db/tenant_collection.py` — `TenantAwareCollection` wrapper enforces tenant isolation on every MongoDB operation
- `app/db/mongodb.py` — six deprecated raw-collection accessors flagged for removal
- `app/services/llm_service.py` — per-process `_response_cache: dict[str, str]` at module level; documented as needing Valkey upgrade
- `app/core/config.py` — `VALKEY_URL`, `SMTP_*` fields already present but not used for Valkey cache or magic links
- Widget — `ConsultationCTA` button has a TODO for the contractor contact link destination

**What v1.1 must add or change:**

| Feature | New or Modified | Scope |
|---------|-----------------|-------|
| Email magic links for customer feedback | NEW service, NEW endpoint, NEW collection | Backend |
| Calibration dashboard aggregations | NEW service, NEW endpoints | Backend |
| Valkey LLM response cache | MODIFY `llm_service.py` | Backend |
| Shared backend utilities package | NEW `packages/efofx-shared/` | Monorepo |
| Shared frontend components library | NEW `packages/efofx-ui/` | Monorepo |
| Tech debt cleanup (INT-04, INT-05, deprecated accessors, ConsultationCTA) | MODIFY existing files | Backend + Widget |

---

## Standard Architecture

### System Overview (v1.1 Delta)

```
┌────────────────────────────────────────────────────────────────────┐
│                     External Layer                                  │
│  ┌───────────────────┐  ┌───────────────────┐  ┌────────────────┐  │
│  │  Contractor Site  │  │  Calibration      │  │  Customer      │  │
│  │  (widget embed)   │  │  Dashboard (NEW)  │  │  Email (NEW)   │  │
│  └────────┬──────────┘  └────────┬──────────┘  └───────┬────────┘  │
│           │ API key               │ JWT                  │ magic URL │
└───────────┼──────────────────────┼──────────────────────┼───────────┘
            │                      │                      │
┌───────────▼──────────────────────▼──────────────────────▼───────────┐
│                  FastAPI — apps/efofx-estimate/                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Existing middleware: TenantAwareCORS, TrustedHost,          │   │
│  │  rate limit headers, process-time headers                    │   │
│  └──────────────────────────────┬───────────────────────────────┘   │
│                                 │                                    │
│  ┌──────────┐ ┌──────────┐ ┌───▼──────┐ ┌──────────┐ ┌──────────┐  │
│  │/auth     │ │/estimate │ │/chat     │ │/feedback │ │/widget   │  │
│  │(v1.0)    │ │(v1.0)    │ │(v1.0)    │ │EXTEND v1.1│ │(v1.0)   │  │
│  └──────────┘ └──────────┘ └──────────┘ └────┬─────┘ └──────────┘  │
│                                               │                     │
│  ┌────────────────────────────────────────────▼────────────────┐    │
│  │  NEW in v1.1:                                               │    │
│  │  /feedback/magic-link/send        → FeedbackEmailService    │    │
│  │  /feedback/magic-link/validate    → one-time token check    │    │
│  │  /calibration/summary             → CalibrationService      │    │
│  │  /calibration/accuracy            → CalibrationService      │    │
│  └─────────────────────────────────────────────────────────────┘    │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                     Service Layer                                     │
│  ┌──────────────────────────────────────────────────────────────┐    │
│  │  v1.0 services unchanged:                                    │    │
│  │  FeedbackService, EstimationService, ChatService,            │    │
│  │  LLMService*, AuthService, BYOKService, WidgetService        │    │
│  │  * LLMService modified: per-process dict → Valkey cache      │    │
│  └──────────────────────────────────────────────────────────────┘    │
│  ┌──────────────────────────────────────────────────────────────┐    │
│  │  NEW in v1.1:                                                │    │
│  │  FeedbackEmailService   — magic link token + email dispatch  │    │
│  │  CalibrationService     — estimate accuracy aggregation      │    │
│  │  ValkeyCache            — shared cache client (singleton)    │    │
│  └──────────────────────────────────────────────────────────────┘    │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                    Persistence Layer                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │ tenants  │ │estimates │ │chat_     │ │feedback  │ │magic_    │   │
│  │          │ │(prompt_  │ │sessions  │ │(actual   │ │link_     │   │
│  │          │ │version)  │ │          │ │outcomes) │ │tokens    │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│              MongoDB Atlas — Motor async driver                      │
│  ┌──────────────────────────────────────────────────────────────┐    │
│  │  NEW: Valkey (DigitalOcean Managed Caching for Valkey)       │    │
│  │  LLM response cache, keyed by SHA-256(messages + model)      │    │
│  └──────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                    External Services                                  │
│  ┌──────────────┐  ┌──────────────────────────────────────────────┐  │
│  │  OpenAI API  │  │  SMTP / fastapi-mail (already wired in v1.0) │  │
│  │  (BYOK)      │  │  Extend: add magic link email template       │  │
│  └──────────────┘  └──────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Status | Responsibility |
|-----------|--------|----------------|
| `FeedbackService` | v1.0 exists | Stores feedback docs; read aggregation (summary, analytics) |
| `FeedbackEmailService` | NEW v1.1 | Generate HMAC magic link tokens; dispatch email via fastapi-mail; store token doc |
| `CalibrationService` | NEW v1.1 | Aggregate actual vs. estimated outcomes; compute accuracy metrics per reference class/region |
| `ValkeyCache` | NEW v1.1 | Singleton async Valkey client; wraps `valkey-py` async connection pool |
| `LLMService` | MODIFY | Swap `_response_cache: dict` for `ValkeyCache`; same SHA-256 cache key logic |
| `magic_link_tokens` collection | NEW v1.1 | One-use token documents with TTL index |
| `packages/efofx-shared/` | NEW v1.1 | Extracted Python utilities (crypto, validation, calculation) as installable package |
| `packages/efofx-ui/` | NEW v1.1 | Extracted React components (EstimateCard, ChatBubble, etc.) as importable package |

---

## Recommended Project Structure

### Backend Changes

```
apps/efofx-estimate/
├── app/
│   ├── api/
│   │   ├── routes.py              MODIFY — add calibration endpoints, magic link endpoints
│   │   ├── auth.py                unchanged
│   │   └── widget.py              unchanged
│   ├── services/
│   │   ├── llm_service.py         MODIFY — replace _response_cache dict with ValkeyCache
│   │   ├── feedback_service.py    unchanged (read/write feedback docs)
│   │   ├── feedback_email_service.py   NEW — magic link generation + email
│   │   ├── calibration_service.py      NEW — accuracy metric aggregations
│   │   └── valkey_cache.py             NEW — singleton cache client
│   ├── models/
│   │   ├── feedback.py            MODIFY — add magic_link_token field, actual_cost captured
│   │   └── calibration.py         NEW — CalibrationSummary, AccuracyMetric Pydantic models
│   ├── db/
│   │   └── mongodb.py             MODIFY — remove deprecated accessors; add magic_link_tokens index
│   └── utils/
│       ├── calculation_utils.py   unchanged (will be mirrored in shared package)
│       ├── validation_utils.py    unchanged (will be mirrored in shared package)
│       └── crypto.py              unchanged (will be mirrored in shared package)
├── requirements.txt               MODIFY — add valkey[asyncio]
└── env.example                    MODIFY — add VALKEY_URL, MAGIC_LINK_SECRET
```

### Shared Libraries (New Monorepo Packages)

```
packages/
├── efofx-shared/                  NEW — Python shared utilities
│   ├── pyproject.toml             package name: efofx-shared, no external deps
│   ├── efofx_shared/
│   │   ├── __init__.py
│   │   ├── crypto.py              extracted from apps/efofx-estimate/app/utils/crypto.py
│   │   ├── validation_utils.py    extracted from apps/efofx-estimate/app/utils/validation_utils.py
│   │   └── calculation_utils.py   extracted from apps/efofx-estimate/app/utils/calculation_utils.py
│   └── tests/
│       └── test_crypto.py
│
└── efofx-ui/                      NEW — React shared components (future vertical reuse)
    ├── package.json               name: @efofx/ui, peerDeps: react, react-dom
    ├── src/
    │   ├── index.ts               re-exports all components
    │   ├── EstimateCard.tsx        extracted from apps/efofx-widget/src/components/
    │   ├── ChatBubble.tsx          extracted from apps/efofx-widget/src/components/
    │   └── TypingIndicator.tsx     extracted from apps/efofx-widget/src/components/
    └── tsconfig.json
```

### Widget Changes

```
apps/efofx-widget/src/
├── components/
│   ├── ConsultationCTA.tsx         MODIFY — wire button destination from branding config
│   └── [other components]          unchanged
└── package.json                    MODIFY — add @efofx/ui workspace dependency (after extraction)
```

### Structure Rationale

- **`FeedbackEmailService` separate from `FeedbackService`:** Email dispatch has different failure semantics (fire-and-forget, SMTP errors shouldn't fail feedback retrieval). Separation avoids coupling.
- **`CalibrationService` as new service:** Aggregation queries are expensive and will gain indexes over time. Isolating them prevents coupling to feedback CRUD.
- **`valkey_cache.py` as module-level singleton:** The async connection pool must survive across requests. Module-level singleton matches how `_response_cache` dict works now — same lifetime, different backing store.
- **`packages/efofx-shared/` with no external deps:** The goal is reuse across future verticals. Zero external dependencies keeps the package lightweight. `pip install -e packages/efofx-shared` in each app's requirements handles local linkage.
- **`packages/efofx-ui/` with peerDeps only:** The UI package should not bundle React — consuming apps provide it. Vite library mode builds the package; `workspace:*` in consuming app's `package.json` links it locally.

---

## Architectural Patterns

### Pattern 1: Magic Link Token — HMAC-Signed, Single-Use

**What:** After an estimate is completed, `FeedbackEmailService` generates a token encoding `{feedback_id, tenant_id, expiry}` signed with HMAC-SHA256 using a server-side secret. The token is stored in `magic_link_tokens` collection with a TTL index and a `used: false` flag. The `/feedback/magic-link/validate` endpoint verifies the signature, checks expiry, and does an atomic `findOneAndUpdate` to flip `used: true`. Replay is blocked structurally.

**When to use:** Customer feedback collection only. No other authentication use case.

**Trade-offs:** No customer login required (good UX). Token must be in the URL (visible in server logs — acceptable because it's a feedback form, not financial data). Single-use prevents replay. 72-hour expiry balances usability vs. staleness risk.

**Example:**
```python
# app/services/feedback_email_service.py
import hmac
import hashlib
import base64
import json
from datetime import datetime, timedelta, timezone

class FeedbackEmailService:

    def generate_magic_token(self, feedback_id: str, tenant_id: str) -> str:
        expiry = int(
            (datetime.now(timezone.utc) + timedelta(hours=72)).timestamp()
        )
        payload = json.dumps(
            {"fid": feedback_id, "tid": tenant_id, "exp": expiry},
            separators=(",", ":"),
            sort_keys=True,
        )
        sig = hmac.new(
            settings.MAGIC_LINK_SECRET.encode(),
            payload.encode(),
            hashlib.sha256,
        ).hexdigest()
        combined = f"{payload}:{sig}"
        return base64.urlsafe_b64encode(combined.encode()).decode()

    async def send_feedback_email(
        self, customer_email: str, token: str, tenant_name: str
    ) -> None:
        """Dispatch magic link email via fastapi-mail (already in v1.0 auth service)."""
        feedback_url = f"{settings.APP_BASE_URL}/feedback?token={token}"
        # ... construct message, call FastMail.send_message()
        # fire-and-forget: called via asyncio.create_task() from route handler

# magic_link_tokens collection document:
# {
#   "_id": ObjectId,
#   "token": "<base64>",
#   "feedback_id": "...",
#   "tenant_id": "...",
#   "used": false,
#   "created_at": ISODate,
#   "expires_at": ISODate   ← TTL index on this field
# }
```

### Pattern 2: Valkey Cache as Module-Level Singleton

**What:** Replace `_response_cache: dict[str, str] = {}` in `llm_service.py` with a module-level `ValkeyCache` instance backed by `valkey-py` (the `valkey` PyPI package, which is a direct fork of `redis-py` with full async support). Cache keys remain the same SHA-256 hash of `(messages + model)`. Values are JSON strings of `EstimationOutput.model_dump_json()`. TTL on cache entries: 24 hours.

**When to use:** LLM estimation calls only (the `generate_estimation` method). Do not cache streaming narrative responses — they are personalized per session.

**Trade-offs:** Valkey is Redis-compatible; `VALKEY_URL` is already in `app/core/config.py`. DigitalOcean Managed Caching for Valkey is a $15/month add-on with a 1GB minimum. For multi-worker deployments, this replaces per-process dict that caused cache misses across workers. Cache survives app restarts.

**Example:**
```python
# app/services/valkey_cache.py
import valkey.asyncio as valkey_async
from app.core.config import settings

_pool: valkey_async.ConnectionPool | None = None

async def get_cache() -> valkey_async.Valkey:
    global _pool
    if _pool is None:
        _pool = valkey_async.ConnectionPool.from_url(settings.VALKEY_URL)
    return valkey_async.Valkey(connection_pool=_pool)

# app/services/llm_service.py — replace _response_cache dict
async def generate_estimation(self, ...) -> EstimationOutput:
    cache_key = _make_cache_key(messages, settings.OPENAI_MODEL)
    cache = await get_cache()
    cached = await cache.get(cache_key)
    if cached:
        return EstimationOutput.model_validate_json(cached)
    # ... call OpenAI ...
    await cache.set(cache_key, result.model_dump_json(), ex=86400)  # 24h TTL
    return result
```

### Pattern 3: Calibration via MongoDB Aggregation Pipeline

**What:** `CalibrationService` queries the `feedback` collection for records that have `actual_cost` set, joins with `estimates` via `estimation_session_id`, and computes accuracy metrics: `(actual - p50) / p50 * 100` for cost accuracy, `(actual - p50) / p50 * 100` for timeline. Groups by `reference_class` and `region` to surface systematic bias.

**When to use:** Called by the calibration dashboard endpoints (`GET /calibration/summary`, `GET /calibration/accuracy`). Not called on every feedback submission — data is computed on demand (MVP scale). Add caching in a later phase if query time exceeds 500ms.

**Trade-offs:** On-demand aggregation is simple and correct. At 15 tenants and 100k estimates/month, the `feedback` collection will have at most thousands of rows with `actual_cost` set (most customers don't report back). Aggregation will be fast without pre-computation. This avoids the complexity of maintaining a separate calibration snapshot collection.

**Example:**
```python
# app/services/calibration_service.py
class CalibrationService:

    async def get_accuracy_metrics(self, tenant: Tenant) -> list[dict]:
        """Compute P50 cost accuracy by reference class for this tenant."""
        col = get_tenant_collection(DB_COLLECTIONS["FEEDBACK"], tenant.tenant_id)
        pipeline = [
            {"$match": {"actual_cost": {"$exists": True, "$ne": None}}},
            {"$lookup": {
                "from": "estimates",
                "localField": "estimation_session_id",
                "foreignField": "session_id",
                "as": "estimate",
            }},
            {"$unwind": "$estimate"},
            {"$group": {
                "_id": {
                    "reference_class": "$estimate.reference_class",
                    "region": "$estimate.region",
                },
                "sample_count": {"$sum": 1},
                "avg_cost_error_pct": {
                    "$avg": {
                        "$multiply": [
                            {"$divide": [
                                {"$subtract": ["$actual_cost", "$estimate.result.total_cost_p50"]},
                                "$estimate.result.total_cost_p50",
                            ]},
                            100,
                        ]
                    }
                },
            }},
            {"$sort": {"sample_count": -1}},
        ]
        cursor = await col.aggregate(pipeline)
        return await cursor.to_list(length=None)
```

### Pattern 4: Shared Python Package via Editable Install

**What:** Extract `app/utils/crypto.py`, `validation_utils.py`, and `calculation_utils.py` into `packages/efofx-shared/`. The existing app continues to work by importing from `efofx_shared` instead of `app.utils`. The package is installed as an editable install (`pip install -e ../../packages/efofx-shared`) so changes reflect immediately without reinstalling.

**When to use:** Any utility that a future vertical (IT/dev estimation app) would also need. Do not over-extract — only functions with zero FastAPI or MongoDB dependencies belong here.

**Trade-offs:** `pyproject.toml` with no external dependencies keeps the package self-contained. Editable install works well in DigitalOcean App Platform if the Dockerfile or build command runs `pip install -e packages/efofx-shared` before the main app install. This is the simplest approach that avoids a private PyPI server.

**Example:**
```toml
# packages/efofx-shared/pyproject.toml
[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.backends.legacy:build"

[project]
name = "efofx-shared"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = []  # zero runtime deps — only stdlib

# apps/efofx-estimate/requirements.txt — add:
# -e ../../packages/efofx-shared
```

### Pattern 5: ConsultationCTA Destination Wiring

**What:** `ConsultationCTA.tsx` has a `// TODO: Open contractor contact link when configured via branding API`. The branding config (`BrandingConfig` type in `widget.d.ts`) needs one new optional field: `consultation_url`. `WidgetService.get_branding_by_prefix()` already returns a `BrandingConfigResponse` — add `consultation_url: Optional[str]` to the Pydantic model and to the `Tenant` document. The widget reads it from `useBranding` context and passes it as a prop to `ConsultationCTA`.

**When to use:** Every estimate result view. When `consultation_url` is null, the button should be hidden or disabled (not a broken no-op like the current console.info).

**Example:**
```typescript
// widget.d.ts — extend BrandingConfig
export interface BrandingConfig {
  // ... existing fields
  consultation_url: string | null;  // NEW
}

// components/ConsultationCTA.tsx
interface Props { consultationUrl: string | null; }
export function ConsultationCTA({ consultationUrl }: Props) {
  if (!consultationUrl) return null;  // hide when not configured
  return (
    <div className="efofx-cta-container">
      <p className="efofx-disclaimer">...</p>
      <a href={consultationUrl} className="efofx-cta-button" target="_blank" rel="noopener noreferrer">
        Request Free Consultation
      </a>
    </div>
  );
}
```

---

## Data Flow

### Magic Link Feedback Flow

```
Estimate completed (widget SSE 'done' event received)
    │
    ▼
Contractor reviews lead → marks project won/lost (manual, post-MVP automation)
    OR
Widget shows "How did we do?" prompt after completion (if configured)
    │
    ▼
POST /api/v1/feedback/magic-link/send
    │  Body: {estimation_session_id, customer_email}
    │  Auth: JWT (contractor is authenticated)
    ▼
FeedbackEmailService.create_and_send(session_id, customer_email, tenant)
    │
    ├─→ Generate HMAC token (payload: feedback_id + tenant_id + 72h expiry)
    ├─→ Insert into magic_link_tokens collection (used: false, expires_at: +72h)
    ├─→ asyncio.create_task(send_magic_link_email(customer_email, token, tenant_name))
    └─→ Return {message: "Feedback email sent", token_id: "..."}

[Customer opens email, clicks link]
    │
    ▼
GET /api/v1/feedback/form?token=<base64_token>  [NO AUTH REQUIRED]
    │
    ├─→ Verify HMAC signature
    ├─→ Check expiry
    ├─→ Find magic_link_tokens doc (used: false) — 404 if not found or already used
    └─→ Return feedback form payload {session_id, reference_class, estimate_summary}

POST /api/v1/feedback/submit-outcome?token=<base64_token>  [NO AUTH REQUIRED]
    │  Body: {actual_cost, actual_timeline_weeks, satisfaction_rating, notes}
    ├─→ Verify token (signature + expiry + used check)
    ├─→ Atomic update: magic_link_tokens.used = true (findOneAndUpdate — prevents replay)
    ├─→ FeedbackService.update_actual_outcome(session_id, tenant_id, outcome_data)
    └─→ Return {message: "Thank you for your feedback"}
```

### Calibration Dashboard Request Flow

```
Contractor logs into dashboard → views calibration page
    │
    ▼
GET /api/v1/calibration/summary
    │  Auth: JWT (contractor authenticated)
    ▼
CalibrationService.get_summary(tenant)
    │
    ├─→ Count feedback docs with actual_cost set (via TenantAwareCollection)
    ├─→ Aggregate: avg accuracy across all reference classes
    ├─→ Aggregate: accuracy trend by week (last 12 weeks)
    └─→ Return CalibrationSummary {
          total_with_outcomes: 42,
          avg_cost_accuracy_pct: -8.3,  # underestimates by 8.3%
          avg_timeline_accuracy_pct: +12.1,  # overestimates by 12.1%
          trend: [{week: "2026-W01", cost_error: -9.1}, ...]
        }

GET /api/v1/calibration/accuracy?group_by=reference_class
    │  Auth: JWT
    ▼
CalibrationService.get_accuracy_metrics(tenant, group_by="reference_class")
    │  MongoDB aggregation: feedback LEFT JOIN estimates ON session_id
    └─→ Return [{reference_class: "residential_pool", region: "SoCal - Coastal",
                 sample_count: 8, avg_cost_error_pct: -12.4, ...}]
```

### Valkey Cache Integration Flow

```
Widget → POST /api/v1/chat/{session_id}/generate-estimate
    │
    ▼
EstimationService.generate_from_chat(session, tenant)
    │
    ├─→ LLMService.generate_estimation(description, reference_class, region)
    │       │
    │       ├─→ cache_key = SHA-256(messages + model)
    │       ├─→ ValkeyCache.get(cache_key)  ← Valkey lookup (async, <1ms)
    │       │       ├─ HIT: return EstimationOutput.model_validate_json(cached)
    │       │       └─ MISS: call OpenAI → store result → return
    │       └─→ Return EstimationOutput
    │
    └─→ Store EstimationSession in MongoDB
```

### Shared Package Import Flow

```
Future vertical app (e.g., apps/efofx-devest/)
    │  requirements.txt: -e ../../packages/efofx-shared
    ▼
from efofx_shared.crypto import encrypt_key, decrypt_key
from efofx_shared.validation_utils import validate_region_code
from efofx_shared.calculation_utils import compute_p80_from_p50
    │
    ▼
Same logic, no duplication, no copy-paste drift
```

---

## New Components Detail

### FeedbackEmailService

**Location:** `apps/efofx-estimate/app/services/feedback_email_service.py`

**Dependencies:** `fastapi-mail` (already in v1.0 auth service for verification emails), `app/core/config.py` (SMTP credentials already wired, `MAGIC_LINK_SECRET` to add), `app/db/mongodb.py` (new `magic_link_tokens` collection)

**New config keys needed:**
- `MAGIC_LINK_SECRET` — random 32-byte secret for HMAC signing (add to `Settings` in `config.py`)
- `FEEDBACK_FORM_URL` — URL of the customer-facing feedback form (could be `APP_BASE_URL` + path)

**New MongoDB collection:** `magic_link_tokens`
- Indexes needed: `expires_at` (TTL, `expireAfterSeconds=0`), `token` (unique), `feedback_id` + `tenant_id` (compound)

### CalibrationService

**Location:** `apps/efofx-estimate/app/services/calibration_service.py`

**Dependencies:** `app/db/tenant_collection.TenantAwareCollection`, `app/db/mongodb.get_tenant_collection`, `app/models/calibration.py` (new Pydantic response models)

**Important constraint:** The `aggregate()` method on `TenantAwareCollection` already prepends a `$match` with `tenant_id` filter. The `$lookup` in the calibration pipeline joins to the `estimates` collection — this join crosses the collection boundary, so the `estimates` side is NOT automatically tenant-scoped by `TenantAwareCollection`. The `$lookup` pipeline must add its own `tenant_id` match in the `pipeline` sub-array to avoid cross-tenant data exposure.

```python
# Correct — scoped lookup
{"$lookup": {
    "from": "estimates",
    "let": {"session_id": "$estimation_session_id", "tid": "$tenant_id"},
    "pipeline": [
        {"$match": {"$expr": {
            "$and": [
                {"$eq": ["$session_id", "$$session_id"]},
                {"$eq": ["$tenant_id", "$$tid"]},  # REQUIRED — join must be tenant-scoped
            ]
        }}}
    ],
    "as": "estimate",
}}
```

### ValkeyCache

**Location:** `apps/efofx-estimate/app/services/valkey_cache.py`

**Dependencies:** `valkey[asyncio]` (add to `requirements.txt`), `app/core/config.py` `VALKEY_URL`

**Lifecycle:** Connection pool initialized lazily on first cache access. Pool is module-level and reused across all requests and workers. Graceful degradation: if Valkey is unavailable, `generate_estimation` must fall through to the OpenAI call rather than failing the request. Wrap Valkey calls in `try/except` and log warnings.

**DigitalOcean integration:** DigitalOcean Managed Caching for Valkey is available as an add-on to App Platform deployments. Connect via the `VALKEY_URL` environment variable set in the App Platform dashboard. TLS is required for production connections; the `valkey-py` client handles TLS when the URL scheme is `rediss://`.

### Shared Python Package

**Location:** `packages/efofx-shared/`

**What gets extracted:**
- `app/utils/crypto.py` — symmetric encryption utilities (Fernet helpers)
- `app/utils/validation_utils.py` — input sanitization, field validators
- `app/utils/calculation_utils.py` — P50/P80 math helpers

**What stays in the app:** All FastAPI-coupled code (service classes, route handlers, Pydantic models with MongoDB aliases). The shared package is pure Python with stdlib only.

**Migration path:** Extract → run existing tests to verify nothing broke → update imports in `app/utils/` to re-export from `efofx_shared` for backward compatibility during transition, then remove the re-exports.

### Shared React Package

**Location:** `packages/efofx-ui/`

**What gets extracted (candidates):**
- `EstimateCard.tsx` — self-contained, no widget-specific state
- `ChatBubble.tsx` — pure presentation
- `TypingIndicator.tsx` — pure presentation

**What stays in the widget:** Components with widget-specific state hooks (`ChatPanel`, `FloatingButton`, `LeadCaptureForm`), Shadow DOM wrapper, context providers.

**Build approach:** Vite library mode (`lib.entry`, `lib.formats: ['es']`). Peer dependencies: `react`, `react-dom`. The widget's `package.json` references `"@efofx/ui": "workspace:*"` via npm/pnpm workspaces.

**Timing note:** This extraction is prep work for a second vertical, not yet consumed by any app. It's safe to do as a pure refactor that doesn't change widget behavior. Verify the widget still builds and renders correctly after extraction.

---

## Tech Debt Items — Integration Points

### INT-04: EstimationSession tenant_id Type

**Current state:** `EstimationSession.tenant_id` field has the wrong type (likely `str` where it should be `PyObjectId` or vice versa, causing inconsistencies with TenantAwareCollection's string-based comparison).

**Fix scope:** `app/models/estimation.py` — change type annotation. Check all write paths (`estimation_service.py`, `chat_service.py`) to ensure the value written matches the type stored. Run existing tests to confirm.

**Integration risk:** Low — TenantAwareCollection uses string comparison; fix must use the same type throughout.

### INT-05: Missing Widget Indexes

**Current state:** `widget_leads` and `widget_analytics` collections lack compound indexes with `tenant_id` as the leftmost field.

**Fix scope:** `app/db/mongodb.py` `create_indexes()` function — add index definitions for these two collections. Pattern matches existing index creation (compound, `tenant_id` first).

**Integration risk:** Zero — purely additive. Indexes are created at startup.

### Deprecated Collection Accessors

**Current state:** Six deprecated accessor functions in `app/db/mongodb.py` (`get_reference_classes_collection`, `get_reference_projects_collection`, `get_estimates_collection`, `get_feedback_collection`, `get_chat_sessions_collection`). All are marked `DEPRECATED: Use get_tenant_collection(...)`.

**Fix scope:** Grep for callers → replace with `get_tenant_collection(...)` calls → remove the deprecated functions.

**Integration risk:** Medium — must audit all call sites. If any admin scripts or tests use these, update them too.

---

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0-15 tenants (current) | Valkey on smallest DO tier ($15/mo, 1GB); calibration on-demand |
| 15-100 tenants | Cache calibration results in Valkey (5-min TTL); add `feedback` indexes for aggregation |
| 100k+ estimates/month | Pre-compute calibration snapshots via background task; Valkey cluster |

### Scaling Priorities

1. **First bottleneck for v1.1:** Calibration aggregation query latency. The `$lookup` across `feedback` and `estimates` collections is an unindexed join on `estimation_session_id` + `tenant_id`. At low data volumes this is fast; add a compound index on `feedback(tenant_id, estimation_session_id)` (already in v1.0) and `estimates(tenant_id, session_id)` (already in v1.0). Query should remain fast up to 100k feedback records.

2. **Second bottleneck for v1.1:** Magic link email delivery latency. SMTP via fastapi-mail is synchronous inside an async context. Use `asyncio.create_task()` to fire-and-forget the email send; return the HTTP response immediately without waiting for SMTP confirmation.

---

## Anti-Patterns

### Anti-Pattern 1: Tenant-Unscoped $lookup in Calibration Aggregation

**What people do:** Write a `$lookup` from `feedback` to `estimates` using only `estimation_session_id` as the join key, trusting that `TenantAwareCollection` already scoped the `feedback` side.

**Why it's wrong:** `TenantAwareCollection.aggregate()` scopes only the source collection (the `$match` prepended to the pipeline). The `$lookup` target (`estimates`) is a raw collection join — it will return rows from any tenant that has a matching `session_id`. At low scale this is invisible; as tenants accumulate, it leaks cross-tenant data into calibration aggregations.

**Do this instead:** Use a correlated `$lookup` with a `pipeline` sub-array that includes `{"$match": {"$expr": {"$eq": ["$tenant_id", "$$caller_tenant_id"]}}}`. Always bind `tenant_id` as a `let` variable and filter it in the inner pipeline.

### Anti-Pattern 2: Magic Link Token Stored in URL Query Parameter Without HTTPS

**What people do:** Assume the magic link is "just a feedback form" and allow HTTP.

**Why it's wrong:** The token is the only authorization for submitting actual project costs (sensitive business data). HTTP exposes it in transit. The token should only be transmitted over HTTPS. DigitalOcean App Platform enforces HTTPS termination at the load balancer — ensure `APP_BASE_URL` in config is `https://`.

**Do this instead:** Set `APP_BASE_URL = https://api.yourdomain.com` in production config. The magic link URL is built from this base. Add a startup assertion that `APP_BASE_URL.startswith("https")` in production environments.

### Anti-Pattern 3: Blocking the Request on Email Send

**What people do:** `await fastmail.send_message(...)` inside the route handler, so the contractor waits for SMTP handshake before getting a response.

**Why it's wrong:** SMTP can take 500ms-2s. The contractor experience degrades with no benefit — whether the email sent successfully does not affect the response the contractor receives.

**Do this instead:** `asyncio.create_task(feedback_email_service.send_magic_link_email(...))`. The task runs in the background event loop. Log errors in the task's exception handler; don't propagate them to the caller.

### Anti-Pattern 4: Shared Package With FastAPI or MongoDB Imports

**What people do:** Extract too broadly — include service base classes or Pydantic models that import from `fastapi` or `motor` in `efofx-shared`.

**Why it's wrong:** The shared package gains transitive dependencies on FastAPI, Motor, and Pydantic that every future consumer must also install. It couples two apps at the framework level, not the logic level.

**Do this instead:** The shared package must import from stdlib only (`hashlib`, `hmac`, `base64`, `math`, `re`). If a function needs Pydantic, it belongs in the app, not the shared package.

### Anti-Pattern 5: Valkey Cache Without Graceful Fallback

**What people do:** Call `await cache.get(key)` without try/except, assuming Valkey is always reachable.

**Why it's wrong:** If DigitalOcean Managed Valkey has a blip, all estimate generation fails — the LLM call never happens. Cache unavailability should degrade to slower (uncached) behavior, not outright failure.

**Do this instead:**
```python
try:
    cached = await cache.get(cache_key)
except Exception:
    logger.warning("Valkey unavailable — proceeding uncached")
    cached = None
```

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| DigitalOcean Managed Valkey | `valkey[asyncio]` Python client, `VALKEY_URL` from env | TLS connection via `rediss://` URL scheme; `valkey` package is a drop-in replacement for `redis-py` |
| fastapi-mail + SMTP | Already used for email verification in v1.0 auth service | Reuse same `FastMail` config; add magic link email template; fire-and-forget via `asyncio.create_task()` |
| OpenAI API | Unchanged — BYOK per-tenant injection via `get_llm_service` dependency | Only the cache backing changes (Valkey replaces in-process dict) |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `FeedbackEmailService` → `magic_link_tokens` collection | Direct Motor insert via `get_database()` | Not tenant-scoped (tokens are indexed by token value, not tenant); still stores `tenant_id` field for audit |
| `FeedbackEmailService` → fastapi-mail | `asyncio.create_task()` fire-and-forget | Do not use `BackgroundTasks` (it runs after response but blocks the event loop for I/O) |
| `CalibrationService` → `feedback` collection | `TenantAwareCollection.aggregate()` | Must scope `$lookup` inner pipeline manually |
| `LLMService` → `ValkeyCache` | Async get/set with graceful fallback | Import `get_cache()` from `valkey_cache.py`; connection pool shared across requests |
| `packages/efofx-shared` → app code | Import from `efofx_shared.*` | Editable install; no circular imports; no FastAPI/Motor imports in the shared package |
| `packages/efofx-ui` → widget | `@efofx/ui` workspace package | Consumed via `import { EstimateCard } from '@efofx/ui'`; widget's Vite config resolves workspace package |

---

## Build Order for v1.1

Dependencies between the v1.1 features determine the safe build order:

**Phase 1 — Tech debt (zero-dependency cleanup)**
- Remove deprecated collection accessors from `mongodb.py` (grep callers first)
- Fix INT-04 (EstimationSession tenant_id type)
- Add INT-05 widget indexes to `create_indexes()`
- YAGNI pass (delete unused code flagged in audit)
- Wire `ConsultationCTA` button destination via branding config

**Phase 2 — Infrastructure (Valkey cache)**
- Add `valkey[asyncio]` to requirements
- Implement `valkey_cache.py` singleton
- Modify `llm_service.py` to use ValkeyCache (same cache key logic, add fallback)
- Provision DigitalOcean Managed Valkey, set `VALKEY_URL` env var

**Phase 3 — Feedback email (depends on: Phase 1 for clean models)**
- Add `MAGIC_LINK_SECRET`, `FEEDBACK_FORM_URL` to config
- Create `magic_link_tokens` collection schema + indexes
- Implement `FeedbackEmailService` (token generation + email)
- Add magic link endpoints to `routes.py`
- Extend `feedback.py` model with `actual_cost`, `actual_timeline` output capture

**Phase 4 — Calibration (depends on: Phase 3 for feedback data model)**
- Create `calibration.py` Pydantic models
- Implement `CalibrationService` (aggregation queries with scoped `$lookup`)
- Add calibration endpoints to `routes.py`

**Phase 5 — Shared library extraction (depends on: Phase 1 for clean codebase)**
- Extract `packages/efofx-shared/` Python utilities
- Update imports in `app/utils/` to use `efofx_shared.*`
- Extract `packages/efofx-ui/` React components
- Update widget imports to use `@efofx/ui`
- Verify both apps build and tests pass

---

## Sources

- Valkey Python client (valkey-py): [GitHub — valkey-io/valkey-py](https://github.com/valkey-io/valkey-py) (HIGH confidence — official Valkey organization repo)
- DigitalOcean Managed Caching for Valkey: [DigitalOcean Valkey Docs](https://docs.digitalocean.com/products/databases/valkey/) (HIGH confidence — official DO docs)
- DigitalOcean Managed Redis → Valkey migration: [DO Blog — Introducing Managed Valkey](https://www.digitalocean.com/blog/introducing-managed-valkey) (HIGH confidence — official announcement)
- fastapi-mail docs: [FastApi-MAIL](https://sabuhish.github.io/fastapi-mail/) (MEDIUM confidence — library docs; already proven in v1.0 codebase)
- Magic link pattern in FastAPI: [FastAPI Passwordless](https://www.scalekit.com/blog/fastapi-passwordless-magic-link-otp-implementation) (MEDIUM confidence — community article; pattern matches v1.0 verification token implementation)
- Python monorepo editable installs: [UV workspaces](https://dev.to/jimmyyeung/journey-migrating-to-uv-workspace-34a7) (MEDIUM confidence — community source; pip editable installs are stdlib behavior)
- Vite library mode for shared UI: [React Monorepo Setup with pnpm and Vite](https://dev.to/lico/react-monorepo-setup-tutorial-with-pnpm-and-vite-react-project-ui-utils-5705) (MEDIUM confidence — community article)
- Existing v1.0 source code: `apps/efofx-estimate/app/` (HIGH confidence — authoritative; read directly)

---

*Architecture research for: efOfX v1.1 Feedback & Quality milestone — integration of magic links, calibration, Valkey, and shared libraries*
*Researched: 2026-02-28*
