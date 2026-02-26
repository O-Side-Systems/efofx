# Architecture Research

**Domain:** Multi-tenant SaaS estimation platform — FastAPI + MongoDB + React widget extension
**Researched:** 2026-02-26
**Confidence:** HIGH (existing codebase reviewed directly; patterns verified against official docs and current sources)

---

## Context: What Already Exists (Epics 1-2 Complete)

The system is brownfield. This research covers the **new components** needed for Epics 3-7 and how they integrate with the established architecture.

**Already built and working:**
- FastAPI layered architecture: `app/api/routes.py` → `app/services/` → `app/db/mongodb.py`
- MongoDB Atlas with Motor async driver
- React widget scaffolded with Shadow DOM isolation via `ShadowDOMWrapper`
- JWT auth skeleton in `app/core/security.py` (partially implemented)
- Tenant model in `app/models/tenant.py` (structure exists, BYOK not wired)
- RCF engine producing P50/P80 estimates from synthetic reference classes
- Service stubs: `EstimationService`, `ChatService`, `FeedbackService`, `LLMService`, `TenantService`

**Gap:** The stubs are scaffolding. The real implementations for multi-tenant isolation, BYOK encryption, prompt management, widget branding, magic-link feedback, and calibration are all missing. Epics 3-7 fill these gaps.

---

## Standard Architecture

### System Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                     External Access Layer                           │
│  ┌──────────────────┐        ┌──────────────────────────────┐      │
│  │  Contractor Site │        │   Tenant Admin Dashboard     │      │
│  │  (host page)     │        │   (future / post-MVP)        │      │
│  └────────┬─────────┘        └──────────────┬───────────────┘      │
│           │ <script embed>                   │ REST/JWT             │
└───────────┼──────────────────────────────────┼────────────────────-┘
            │                                  │
┌───────────▼──────────────────────────────────▼─────────────────────┐
│                      Widget Layer (Epic 5)                           │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  efofx-widget (React 19 + Vite, Shadow DOM isolation)        │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐   │   │
│  │  │ Chat UI  │  │ Lead     │  │ Estimate │  │ Branding  │   │   │
│  │  │ Component│  │ Capture  │  │ Results  │  │ Config    │   │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬─────┘   │   │
│  │       └─────────────┴─────────────┴───────────────┘         │   │
│  │                  API Client (fetch + session token)          │   │
│  └──────────────────────────────┬──────────────────────────────┘   │
└─────────────────────────────────┼───────────────────────────────────┘
                                  │ HTTPS / JSON
┌─────────────────────────────────▼───────────────────────────────────┐
│                      API Layer — FastAPI                             │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  TenantIsolationMiddleware  ← extracts tenant_id from JWT  │    │
│  │  RateLimitMiddleware        ← per-tenant tier enforcement   │    │
│  │  CORSMiddleware             ← host-page whitelist           │    │
│  └──────────────────────────────┬─────────────────────────────┘    │
│                                 │                                    │
│  ┌──────────┐ ┌──────────┐ ┌───▼──────┐ ┌──────────┐ ┌──────────┐ │
│  │/tenants  │ │/estimate │ │/chat     │ │/feedback │ │/widget   │ │
│  │(Epic 3)  │ │(Epics 2) │ │(Epic 4)  │ │(Epic 6)  │ │/config   │ │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ │
└───────┼────────────┼────────────┼─────────────┼────────────┼───────┘
        │            │            │             │            │
┌───────▼────────────▼────────────▼─────────────▼────────────▼───────┐
│                     Service Layer                                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ Tenant   │ │Estimation│ │  LLM     │ │ Feedback │ │ Widget   │ │
│  │ Service  │ │ Service  │ │ Service  │ │ Service  │ │ Config   │ │
│  │(Epic 3)  │ │+RCFEngine│ │(Epic 4)  │ │(Epic 6)  │ │ Service  │ │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ │
│       │            │       ┌────┴────┐        │            │       │
│  ┌────┴──────────────────────────────────────────────────────────┐ │
│  │  BYOK Key Service (Epic 3) — Fernet encrypt/decrypt at rest   │ │
│  └────────────────────────────────────────────────────────────────┘ │
└───────┬────────────┬────────────┬─────────────┬────────────┬───────┘
        │            │            │             │            │
┌───────▼────────────▼────────────▼─────────────▼────────────▼───────┐
│                    Persistence Layer                                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ tenants  │ │estimates │ │chat_     │ │feedback  │ │widget_   │ │
│  │collection│ │collection│ │sessions  │ │collection│ │configs   │ │
│  │          │ │          │ │collection│ │          │ │collection│ │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ │
│              MongoDB Atlas — Motor async driver                     │
│              All collections: compound index (tenant_id, ...)      │
└─────────────────────────────────────────────────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────────────┐
│                    External Services                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │ OpenAI API   │  │ SendGrid     │  │ DigitalOcean Spaces CDN  │  │
│  │ (per-tenant  │  │ (magic link  │  │ (embed.js distribution)  │  │
│  │  BYOK key)   │  │  emails)     │  │                          │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **TenantIsolationMiddleware** | Extract + validate tenant_id from every JWT; inject into request.state | FastAPI middleware with python-jose |
| **RateLimitMiddleware** | Per-tenant tier-based limits; reject 429 before hitting services | In-memory dict (MVP); Redis post-MVP |
| **TenantService** (Epic 3) | CRUD for tenants, email verification flow, tier management | FastAPI service + MongoDB tenants collection |
| **BYOKKeyService** (Epic 3) | Encrypt tenant OpenAI keys at rest; decrypt at call time only | Fernet (cryptography library), key from env |
| **LLMService** (Epic 4) | BYOK-aware OpenAI calls; prompt version management; response caching | AsyncOpenAI with per-request api_key override |
| **PromptRegistry** (Epic 4) | Load/version prompts from JSON files in git; inject into LLM calls | File loader + dict cache; store prompt_version in estimates |
| **ChatService** (Epic 4) | Conversational scoping engine; multi-turn context management | LLMService + ChatSession MongoDB document |
| **WidgetConfigService** (Epic 5) | Store/retrieve per-tenant branding: colors, logos, text overrides | MongoDB widget_configs collection |
| **FeedbackService** (Epic 6) | Magic-link token generation; actual outcome capture; calibration aggregations | MongoDB feedback collection + SendGrid |
| **CalibrationService** (Epic 6) | Compare P50/P80 estimates vs actual outcomes; tune synthetic data | MongoDB aggregation pipeline |
| **ShadowDOMWrapper** | Isolate widget CSS/JS from host page; inject tenant branding via CSS vars | React component, mode:'open' |
| **WidgetBrandingProvider** | Fetch tenant config on init; set CSS custom properties | React context + API call on mount |

---

## Recommended Project Structure

### Backend Extension (Epic 3-7 additions to existing layout)

```
apps/efofx-estimate/app/
├── api/
│   ├── routes.py                 # existing — extend with new endpoints
│   └── routes/                   # refactor: split by domain (Epic 7)
│       ├── estimation.py
│       ├── chat.py
│       ├── feedback.py
│       ├── tenants.py            # NEW Epic 3
│       └── widget_config.py      # NEW Epic 5
├── core/
│   ├── config.py                 # extend: add ENCRYPTION_KEY, SENDGRID_API_KEY
│   ├── constants.py
│   └── security.py               # extend: BYOK decrypt, tenant_id claims
├── middleware/
│   ├── __init__.py
│   ├── tenant_isolation.py       # NEW Epic 3 — enforce tenant_id on every request
│   └── rate_limiter.py           # EXTEND — per-tier limits
├── models/
│   ├── tenant.py                 # extend: add encrypted_openai_key field, tier
│   ├── estimation.py
│   ├── chat.py
│   ├── feedback.py               # extend: add magic_link_token, actual_cost
│   └── widget_config.py          # NEW Epic 5
├── services/
│   ├── tenant_service.py         # IMPLEMENT Epic 3 (stub exists)
│   ├── byok_key_service.py       # NEW Epic 3
│   ├── llm_service.py            # IMPLEMENT Epic 4 (stub exists — naive)
│   ├── prompt_registry.py        # NEW Epic 4
│   ├── estimation_service.py
│   ├── chat_service.py           # IMPLEMENT Epic 4 (stub exists)
│   ├── feedback_service.py       # IMPLEMENT Epic 6 (stub exists)
│   ├── calibration_service.py    # NEW Epic 6
│   ├── widget_config_service.py  # NEW Epic 5
│   └── rcf_engine.py
├── prompts/                      # NEW Epic 4 — git-versioned prompt files
│   ├── estimate_narrative.json
│   ├── project_classifier.json
│   └── chat_scoping.json
├── db/
│   └── mongodb.py                # extend: add index creation at startup
└── utils/
    ├── calculation_utils.py
    ├── validation_utils.py
    └── magic_link_utils.py       # NEW Epic 6
```

### Widget Extension (Epic 5 additions)

```
apps/efofx-widget/src/
├── main.tsx                      # extend: accept tenantId + apiKey from data-attrs
├── components/
│   ├── ShadowDOMWrapper.tsx      # extend: inject CSS custom properties
│   ├── BrandingProvider.tsx      # NEW — fetch + apply tenant branding
│   ├── ChatInterface.tsx         # NEW — multi-turn conversation UI
│   ├── LeadCaptureForm.tsx       # NEW — name/email before estimate
│   ├── EstimateDisplay.tsx       # NEW — P50/P80 results + breakdown
│   └── LoadingState.tsx          # NEW
├── hooks/
│   ├── useBranding.ts            # NEW — fetch branding config on mount
│   └── useEstimation.ts          # NEW
├── api/
│   └── client.ts                 # extend: add session token, branding endpoint
└── types/
    └── branding.ts               # NEW
```

### Structure Rationale

- **`prompts/`**: Git-versioned JSON files mean prompt changes are reviewable, diffable, and rollbackable. No external dependency needed for MVP. Store `prompt_version` field on every estimate for calibration traceability.
- **`byok_key_service.py`**: Isolated service — decryption only happens here, never in route handlers. Ensures the raw key never appears in logs.
- **`middleware/tenant_isolation.py`**: Middleware (not Depends) ensures the check happens before any route code executes, even if a developer forgets to add the Depends decorator.
- **`widget_config/`**: Separate collection from tenant record — branding is updated frequently without touching auth/billing fields.

---

## Architectural Patterns

### Pattern 1: JWT Tenant Context Propagation

**What:** Every request carries a JWT with `tenant_id` claim. The isolation middleware extracts this and attaches it to `request.state.tenant_id`. All services receive `tenant_id` as their first parameter — never fetch it themselves.

**When to use:** Every authenticated route. No exceptions.

**Trade-offs:** Stateless and fast. Requires discipline — every query must use the tenant_id parameter.

**Example:**
```python
# middleware/tenant_isolation.py
class TenantIsolationMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next):
        token = request.headers.get("Authorization", "").replace("Bearer ", "")
        if token:
            payload = jwt.decode(token, settings.JWT_SECRET_KEY, algorithms=["HS256"])
            request.state.tenant_id = payload.get("tenant_id")
            request.state.tenant_tier = payload.get("tier", "basic")
        response = await call_next(request)
        return response

# service layer — always first parameter
async def get_estimates(tenant_id: str, limit: int = 20):
    return await db.estimates.find(
        {"tenant_id": tenant_id}  # REQUIRED — never omit
    ).limit(limit).to_list(None)
```

### Pattern 2: BYOK Encryption/Decryption at Request Time

**What:** Tenant OpenAI API keys are stored encrypted with Fernet (AES-128 CBC + HMAC-SHA256). The raw key is decrypted only at the moment of the API call, never stored in memory beyond that scope.

**When to use:** All LLM calls that use tenant-provided OpenAI keys.

**Trade-offs:** Adds ~1ms per LLM call for decryption. The encryption key lives in the environment — keep it out of source control.

**Example:**
```python
# services/byok_key_service.py
from cryptography.fernet import Fernet

class BYOKKeyService:
    def __init__(self):
        self._cipher = Fernet(settings.ENCRYPTION_KEY.encode())

    def encrypt_key(self, raw_key: str) -> bytes:
        return self._cipher.encrypt(raw_key.encode())

    def decrypt_key(self, encrypted_key: bytes) -> str:
        return self._cipher.decrypt(encrypted_key).decode()

# services/llm_service.py — use per-request key override
class LLMService:
    async def generate(self, prompt: str, tenant: Tenant) -> str:
        api_key = byok_service.decrypt_key(tenant.openai_api_key_encrypted)
        client = AsyncOpenAI(api_key=api_key)  # ephemeral, not stored
        response = await client.chat.completions.create(
            model="gpt-4o-mini",
            messages=[{"role": "user", "content": prompt}]
        )
        return response.choices[0].message.content
```

### Pattern 3: Git-Based Prompt Registry

**What:** Prompts are JSON files in `app/prompts/`. Each file has a `version` field. The `PromptRegistry` loads all prompts at startup and caches in memory. The `prompt_version` is recorded on every EstimationResult document for calibration tracking.

**When to use:** All LLM calls. No inline prompt strings in service code.

**Trade-offs:** Simple and zero-dependency. Requires app restart to pick up prompt changes (acceptable for MVP — DO auto-deploys on git push).

**Example:**
```python
# services/prompt_registry.py
import json
from pathlib import Path

class PromptRegistry:
    _prompts: dict = {}

    def load_all(self):
        prompts_dir = Path(__file__).parent.parent / "prompts"
        for path in prompts_dir.glob("*.json"):
            data = json.loads(path.read_text())
            self._prompts[path.stem] = data

    def get(self, name: str) -> dict:
        # Returns {"version": "1.2.0", "system": "...", "user_template": "..."}
        return self._prompts[name]

# app/prompts/estimate_narrative.json
{
  "version": "1.2.0",
  "system": "You are an expert construction estimator...",
  "user_template": "Project: {description}\nP50: {p50}\nP80: {p80}\n..."
}
```

### Pattern 4: Widget Branding via CSS Custom Properties

**What:** The widget `init()` accepts a `tenantId` and `apiKey` via data attributes. On mount, it fetches the tenant's branding config from `/api/v1/widget/config/{tenant_id}`. The `ShadowDOMWrapper` sets CSS custom properties on the shadow root, making all tenant colors/fonts available to all child components.

**When to use:** All widget rendering. Branding is applied once at init; components consume via CSS vars.

**Trade-offs:** CSS custom properties pierce Shadow DOM boundary (intentional), so host-page vars do NOT leak in. Single fetch at init; no per-component API calls.

**Example:**
```typescript
// components/ShadowDOMWrapper.tsx
function ShadowDOMWrapper({ children, branding }) {
  const shadowRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (shadowRef.current && branding) {
      const shadow = shadowRef.current.shadowRoot;
      shadow.host.style.setProperty('--efofx-primary', branding.primaryColor);
      shadow.host.style.setProperty('--efofx-font', branding.fontFamily);
    }
  }, [branding]);

  return <div ref={shadowRef}>{children}</div>;
}

// components — consume branding
<button style={{ backgroundColor: 'var(--efofx-primary)' }}>
  Get Estimate
</button>
```

### Pattern 5: Magic Link Feedback Token Flow

**What:** After project completion, the system sends a magic link email (via SendGrid) with a signed, time-limited token. The token encodes `feedback_id + tenant_id + expiry`. No authentication required — the token IS the authorization. Token is stored in MongoDB with single-use flag.

**When to use:** Customer feedback collection (Epic 6). Token grants access to one feedback form, one time.

**Trade-offs:** Simple UX (no login required for end customers). HMAC signing prevents forgery. Single-use prevents replay attacks.

**Example:**
```python
# utils/magic_link_utils.py
import hmac, hashlib, base64
from datetime import datetime, timedelta

def generate_magic_token(feedback_id: str, tenant_id: str) -> str:
    expiry = int((datetime.utcnow() + timedelta(hours=72)).timestamp())
    payload = f"{feedback_id}:{tenant_id}:{expiry}"
    sig = hmac.new(settings.MAGIC_LINK_SECRET.encode(), payload.encode(), hashlib.sha256)
    return base64.urlsafe_b64encode(f"{payload}:{sig.hexdigest()}".encode()).decode()

def verify_magic_token(token: str) -> dict:
    decoded = base64.urlsafe_b64decode(token.encode()).decode()
    parts = decoded.rsplit(":", 1)
    payload, provided_sig = parts[0], parts[1]
    expected_sig = hmac.new(settings.MAGIC_LINK_SECRET.encode(), payload.encode(), hashlib.sha256)
    if not hmac.compare_digest(expected_sig.hexdigest(), provided_sig):
        raise ValueError("Invalid token signature")
    feedback_id, tenant_id, expiry = payload.split(":")
    if int(expiry) < int(datetime.utcnow().timestamp()):
        raise ValueError("Token expired")
    return {"feedback_id": feedback_id, "tenant_id": tenant_id}
```

---

## Data Flow

### Request Flow: LLM Estimate with BYOK (Epic 4)

```
Widget (browser)
    │  POST /api/v1/chat/send  {session_id, message}
    │  Authorization: Bearer <widget_session_token>
    ▼
TenantIsolationMiddleware
    │  Decodes JWT → tenant_id, tier
    │  Attaches to request.state
    ▼
RateLimitMiddleware
    │  Checks per-tenant tier limit
    │  429 if exceeded
    ▼
ChatService.send_message(message, tenant_id)
    │
    ├─→ LLMService.generate(prompt, tenant)
    │       │
    │       ├─→ PromptRegistry.get("chat_scoping") → prompt template + version
    │       ├─→ BYOKKeyService.decrypt_key(tenant.openai_api_key_encrypted)
    │       ├─→ AsyncOpenAI(api_key=decrypted_key).chat.completions.create(...)
    │       └─→ Response text + prompt_version
    │
    ├─→ ChatSession.append_message() → MongoDB chat_sessions
    └─→ Return ChatResponse to widget
```

### Request Flow: Magic Link Feedback (Epic 6)

```
[Trigger: Estimate marked complete by contractor]
    │
    ▼
FeedbackService.create_feedback_request(session_id, tenant_id)
    │  Creates Feedback document (status: pending)
    │
    ├─→ MagicLinkUtils.generate_token(feedback_id, tenant_id)
    ├─→ SendGrid.send_email(customer_email, magic_link_url)
    └─→ Feedback.magic_link_sent_at = now()

[Customer clicks link in email — days later]
    │
    ▼
GET /api/v1/feedback/form?token=<magic_token>  (no auth required)
    │
    ├─→ MagicLinkUtils.verify_token(token) → {feedback_id, tenant_id}
    ├─→ Feedback.find({_id: feedback_id, tenant_id: tenant_id, used: false})
    └─→ Return feedback form HTML/JSON

POST /api/v1/feedback/submit?token=<magic_token>
    │
    ├─→ verify_token() + mark token used (single-use enforcement)
    ├─→ FeedbackService.save_actual_outcome(actual_cost, notes)
    └─→ CalibrationService.trigger_recalculation(tenant_id)
```

### Widget Init Flow (Epic 5)

```
Contractor site loads
    │  <script src="widget.efofx.ai/embed.js"
    │           data-tenant-id="tenant_abc"
    │           data-api-key="efofx_sk_..."></script>
    ▼
widget.init({ tenantId, apiKey })
    │
    ├─→ GET /api/v1/widget/config/{tenant_id}  → branding config
    │       └─→ {primaryColor, fontFamily, logoUrl, welcomeMessage}
    ├─→ ShadowDOMWrapper.attachShadow({mode: 'open'})
    ├─→ CSS custom properties set on shadow host
    ├─→ React render: <BrandingProvider> → <ChatInterface>
    └─→ Widget ready — user sees branded chat bubble
```

### State Management

```
MongoDB (durable state)
    ├── tenants           — tenant records with encrypted BYOK key
    ├── estimates         — EstimationSession with result, prompt_version
    ├── chat_sessions     — ChatSession with messages + LLM context
    ├── feedback          — magic_link_token, actual outcomes, calibration data
    └── widget_configs    — per-tenant branding config

FastAPI Request State (ephemeral per-request)
    ├── request.state.tenant_id
    └── request.state.tenant_tier

In-Memory (service lifetime, lost on restart)
    ├── PromptRegistry._prompts  — prompt templates loaded at startup
    └── RateLimiter.requests     — sliding window counters (lose on restart: acceptable for MVP)
```

---

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0-15 tenants (MVP target) | Current design is fine — single server, in-memory rate limiting |
| 15-100 tenants | Move rate limiting to Redis (avoid drift on restart); add MongoDB read replicas; enable OpenAI prompt caching for shared system prompts |
| 100k+ estimates/month | Add async task queue (Celery/ARQ) for LLM calls to avoid request timeouts; consider Redis cache for tenant branding configs (currently fetched on every widget init) |

### Scaling Priorities

1. **First bottleneck:** OpenAI API latency — LLM calls are 1-5 seconds. The chat endpoint will time out for slow calls. Fix: async task + polling pattern (start job → return job_id → poll for result).
2. **Second bottleneck:** MongoDB connection pool exhaustion if many tenants run concurrent estimates. Fix: Motor connection pool tuning + query profiling on compound indexes.

---

## Anti-Patterns

### Anti-Pattern 1: Missing tenant_id in MongoDB Queries

**What people do:** Write a service method without including `tenant_id` in the query filter, especially in utility/admin code paths.

**Why it's wrong:** Returns data from ALL tenants. Cross-tenant data leakage is a critical security failure, not just a bug. At MVP scale (15 tenants) this is catastrophic.

**Do this instead:** All service methods accept `tenant_id` as the first parameter. Every MongoDB query includes `{"tenant_id": tenant_id, ...}` as the first filter key. Use compound indexes with `tenant_id` as the leftmost field so missing it produces an obvious index miss during profiling.

### Anti-Pattern 2: Instantiating OpenAI Client at Service Init

**What people do:** Create `AsyncOpenAI(api_key=settings.OPENAI_API_KEY)` in the `LLMService.__init__()` — currently done in the existing stub.

**Why it's wrong:** BYOK requires a different key per tenant per request. A single shared client uses the wrong key. Also stores the key in object state across requests.

**Do this instead:** Instantiate `AsyncOpenAI(api_key=decrypted_key)` inside the method that makes the call. The decrypted key lives only in that function's scope.

### Anti-Pattern 3: Inline Prompt Strings

**What people do:** Write prompt text directly in service code: `prompt = f"Analyze the project: {description}"`.

**Why it's wrong:** Can't version or diff prompts. Can't track which prompt version produced which estimate (breaks calibration). Hard to iterate without code changes.

**Do this instead:** All prompts in `app/prompts/*.json` with a `version` field. Store `prompt_version` on every EstimationResult. Change prompts by editing JSON and deploying.

### Anti-Pattern 4: Widget Loads Branding on Every API Call

**What people do:** Fetch tenant branding config in every chat message handler inside the widget.

**Why it's wrong:** Adds 100-300ms latency to every user interaction. Branding doesn't change during a session.

**Do this instead:** Fetch branding once at widget `init()`. Cache in React context for the session lifetime. Invalidate only on explicit reload.

### Anti-Pattern 5: Magic Links Without Single-Use Enforcement

**What people do:** Verify the token signature but don't mark it used in the database.

**Why it's wrong:** Anyone who intercepts or receives a forwarded email can submit fake feedback. Feedback replay corrupts calibration data.

**Do this instead:** Atomic MongoDB update: `{$set: {"used": true, "used_at": now()}}` with `{returnDocument: AFTER}`. If `used` is already true, reject with 409.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| OpenAI API | Per-request BYOK key override via `AsyncOpenAI(api_key=...)` | Decrypt Fernet-encrypted key at call time; never store decrypted key |
| SendGrid | Python SDK (sendgrid 6.12.5) via `FeedbackService` | Magic link emails; tenant notification emails; DigitalOcean blocks direct SMTP |
| DigitalOcean Spaces CDN | S3-compatible boto3 API for widget JS upload; CDN serves embed.js | Cache busting via filename versioning (`embed.v1.2.0.js`) |
| Sentry | `sentry_sdk.capture_exception()` in service catch blocks | Tag all events with `tenant_id` for per-tenant error filtering |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Widget → FastAPI | HTTPS JSON REST with session token | Session token issued at widget init; includes tenant_id and widget_session scope |
| TenantMiddleware → Services | `request.state.tenant_id` passed as parameter | Middleware enriches; services consume as parameter — NOT by reading request state themselves |
| LLMService → PromptRegistry | Direct method call (`registry.get("estimate_narrative")`) | Registry is a singleton loaded at startup; safe for concurrent reads |
| FeedbackService → CalibrationService | Direct method call on feedback submission | Synchronous for MVP; extract to background task if calibration becomes slow |
| FeedbackService → SendGrid | Async HTTP call via `aiohttp` (do not block request thread) | Wrap in `asyncio.create_task()` so email send doesn't block feedback submission response |

---

## Build Order Implications

The architecture has hard dependencies between components. This ordering avoids blocked work:

**Phase 1 — Foundation for everything else (Epic 3: Multi-Tenant + Auth)**
- `TenantIsolationMiddleware` and `BYOKKeyService` must exist before any LLM work
- MongoDB compound indexes on `tenant_id` must be created before any load testing
- JWT token structure with `tenant_id` claim must be final before widget tokens are designed
- No other Epic can be properly implemented without tenant context propagating through every layer

**Phase 2 — LLM backbone (Epic 4: LLM Integration)**
- Depends on: BYOK key service from Phase 1
- `PromptRegistry` can be built standalone; `LLMService` BYOK refactor needs Phase 1
- Estimate narratives and chat scoping are independent of each other within this phase

**Phase 3 — Distribution surface (Epic 5: Widget)**
- Depends on: Chat endpoints from Phase 2; tenant branding model from Phase 1
- Shadow DOM already exists; new work is branding config API + BrandingProvider component
- Widget embed code and CDN setup are independent of chat implementation

**Phase 4 — Feedback loop (Epic 6: Feedback + Calibration)**
- Depends on: Estimates with `prompt_version` field from Phase 2; tenant email from Phase 1
- Magic link system is standalone; calibration depends on real feedback data existing
- Calibration can be stubbed initially and refined as data accumulates

**Phase 5 — Hardening (Epic 7: Code Quality)**
- Depends on: All above working; identifies cleanup targets after implementation
- YAGNI pass on stubs left from Epic 1 that were never properly implemented

---

## Sources

- MongoDB multi-tenant architecture official docs: [Build a Multi-Tenant Architecture - Atlas](https://www.mongodb.com/docs/atlas/build-multi-tenant-arch/)
- MongoDB compound indexes: [Compound Indexes](https://www.mongodb.com/docs/manual/core/indexes/index-types/index-compound/)
- FastAPI dependency injection for multi-tenancy: [PropelAuth FastAPI Auth](https://www.propelauth.com/post/fastapi-auth-with-dependency-injection) (MEDIUM confidence — community source)
- Embeddable React widget production guide: [MakerKit Embeddable Widgets](https://makerkit.dev/blog/tutorials/embeddable-widgets-react) (MEDIUM confidence — verified against Shadow DOM API docs)
- FastAPI multi-tenant patterns 2025: [Multi-Tenant Architecture with FastAPI](https://medium.com/@koushiksathish3/multi-tenant-architecture-with-fastapi-design-patterns-and-pitfalls-aa3f9e75bf8c) (LOW confidence — Medium article, patterns align with official FastAPI Depends docs)
- Fernet encryption docs: [Fernet — cryptography 47.0 docs](https://cryptography.io/en/latest/fernet/) (HIGH confidence — official library docs)
- Magic link architecture: [FastAPI Passwordless](https://www.scalekit.com/blog/fastapi-passwordless-magic-link-otp-implementation) (MEDIUM confidence)
- Shadow DOM CSS custom properties: [React Shadow DOM Web Components](https://the-expert-developer.medium.com/%EF%B8%8F-react-shadow-dom-web-components-real-world-interop-styling-events-and-forms-4f52b26e4b32) (MEDIUM confidence)
- Existing project architecture doc: `/Users/brettlee/work/efofx-workspace/docs/architecture.md` (HIGH confidence — authoritative project source)

---
*Architecture research for: efOfX multi-tenant SaaS estimation platform — Epics 3-7*
*Researched: 2026-02-26*
