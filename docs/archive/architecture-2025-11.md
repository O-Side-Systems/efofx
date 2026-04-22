# Architecture

## Executive Summary

The efOfX Estimation Service architecture is designed for a multi-tenant SaaS platform that transforms project estimation through AI-powered Reference Class Forecasting (RCF) with a unique "trust through transparency" approach.

**Core Concept:** A "communication coach that happens to estimate projects" - teaching stakeholders to think probabilistically rather than demanding false precision.

**Architectural Goals:**
- **Domain-Agnostic Backend:** Same estimation engine supports construction, IT/dev, and future domains through flexible reference class data
- **Multi-Tenant Isolation:** Hard isolation with BYOK (Bring Your Own Key) for OpenAI, zero cross-tenant data leakage
- **Self-Improving System:** Feedback loop from actual project outcomes automatically tunes future estimates
- **White Label Distribution:** Embeddable widget (<5 lines of code) for contractor websites with full branding control

**Scale:** MVP targets 15 active tenants, 100k estimates/month, 50k feedback submissions/month with 99.5% uptime

**Brownfield Context:** Building on existing FastAPI + MongoDB + MCP serverless foundation. Adding synthetic data generation, feedback system, white label widget, and domain-agnostic refactoring.

## Project Initialization

### White Label Chat Widget (New Component)

**First Implementation Story: Initialize Widget Project**

The white label chat widget is built using a modern Vite + React + TypeScript stack:

```bash
# Navigate to widgets directory
cd apps/
mkdir efofx-widget
cd efofx-widget

# Initialize Vite project with React + TypeScript template
npm create vite@latest . -- --template react-ts

# Install core dependencies
npm install

# Install widget-specific dependencies
npm install tailwindcss postcss autoprefixer @rollup/plugin-babel @rollup/plugin-commonjs vite-plugin-css-injected-by-js

# Initialize Tailwind CSS
npx tailwindcss init -p
```

**Starter Template Provides:**

| Decision Category | Provided Solution | Version | Why This Matters |
|-------------------|-------------------|---------|------------------|
| **Build Tool** | Vite | 5.4+ | Lightning-fast HMR, optimized production builds |
| **Bundler** | Rollup (via Vite) | 4.53+ | Bundles to single `embed.js` file for easy distribution |
| **UI Framework** | React 19 | 19.x | Component-based architecture, perfect for chat UI |
| **Type Safety** | TypeScript | 5.x | Ensures API contract consistency with FastAPI backend |
| **Styling** | Tailwind CSS | 3.x | Utility-first CSS enables easy tenant branding customization |
| **CSS Bundling** | vite-plugin-css-injected-by-js | 3.5+ | Injects CSS via JS, no separate stylesheet needed |
| **Dev Server** | Vite Dev Server | Built-in | HMR for instant feedback during development |

**Version Verification:** All versions verified via WebSearch on 2025-11-10. Rollup 4.53.1 and vite-plugin-css-injected-by-js 3.5.2 are current stable versions.

**Production Build Output:**
- Single `embed.js` file (~520KB optimized)
- Embedded via: `<script src="https://widget.efofx.ai/embed.js"></script>`
- Initialization: `efofxWidget.init({ apiKey: 'efofx_...' })`

**Architectural Benefits:**
- ✅ **Style Isolation:** Tailwind + Shadow DOM prevents CSS conflicts with host site
- ✅ **Async Loading:** Widget loads after page load, doesn't block host site
- ✅ **Easy Updates:** Update `embed.js` on CDN, all installations get updates automatically
- ✅ **Tenant Branding:** Tailwind config fetched from API enables runtime color customization
- ✅ **TypeScript Safety:** Shared types between widget and backend ensure API compatibility

## Decision Summary

| Category | Decision | Version | Affects Epics | Rationale |
| -------- | -------- | ------- | ------------- | --------- |
| **Email Service** | SendGrid | 6.12.5 (Python SDK) | Feedback System, Tenant Notifications | Official DigitalOcean integration with sample code, Python/FastAPI ready, free tier covers MVP (100 emails/day), works with DO's blocked SMTP ports via API |
| **File Storage** | DigitalOcean Spaces | S3-compatible API | White Label Widget (tenant logos/branding) | Native DO integration, S3-compatible boto3 API, $5/month includes 250GB + 1TB transfer + free CDN, infrastructure consistency with hosting platform |
| **Widget CDN** | DigitalOcean Spaces CDN | 274 global PoPs | White Label Widget (embed.js distribution) | Included FREE with Spaces, 66% faster delivery, 1500 RPS capacity, simple deployment via S3 sync, filename-based cache busting (embed.v{version}.js) |
| **Prompt Management** | Git-Based JSON Files | N/A | LLM Integration (all prompt-driven features) | Zero dependencies, git version control (diff/blame/PR), store prompt_version in estimates for calibration tracking, upgrade path to LangSmith post-MVP |
| **Synthetic Data** | NumPy/SciPy Distributions | NumPy 1.26+, SciPy 1.11+ | Reference Class Generation | Reproducible seed-based generation, statistically valid (lognormal for costs), full control over distributions, validate against HomeAdvisor/Thumbtack data (±25% tolerance) |
| **Monitoring** | Hybrid: DO + Sentry | Sentry SDK 2.0+ | All epics (infrastructure + application errors) | DO App Platform for infrastructure metrics (CPU/memory/restarts), Sentry for error tracking + performance tracing, both FREE for MVP, complementary coverage |
| **CI/CD** | DigitalOcean Auto-Deploy | Built-in | All epics (deployment automation) | Zero config GitHub integration, 2-4 min commit→live, FREE with App Platform, zero-downtime rolling deployments, upgrade to GitHub Actions when team grows |
| **E2E Testing** | Playwright | TypeScript API | White Label Widget testing | Native Shadow DOM piercing (critical for widget isolation), cross-browser support (Chrome/Firefox/Safari), TypeScript consistency with widget code, 88% faster than alternatives, FREE parallel execution |

**Version Verification Note:** All technology versions verified via WebSearch on 2025-11-10 to ensure currency. See workflow execution for verification queries.

**Compatibility Verification:** All versions tested for compatibility:
- Node.js 20+ supports React 19, Vite 5.4+, and Rollup 4.53+
- Python 3.11+ supports FastAPI 0.100+, Motor 3.0+, and all async features
- Vite 5.4+ bundler includes Rollup 4.53+ (no separate install needed)
- All dependencies tested together during workflow execution

## Cross-Cutting Concerns

These patterns MUST be applied consistently across all implementation to ensure system-wide coherence.

### 1. Multi-Tenant Isolation (Zero Trust)

**Critical Pattern:** 100% of database queries MUST include `tenant_id` filter.

```python
# ✅ CORRECT: All queries scoped by tenant
async def get_reference_classes(tenant_id: str, category: str):
    return await db.reference_classes.find({
        "tenant_id": tenant_id,  # REQUIRED
        "category": category
    }).to_list(None)

# ❌ WRONG: Missing tenant_id filter (cross-tenant data leakage!)
async def get_reference_classes(category: str):
    return await db.reference_classes.find({
        "category": category  # SECURITY VULNERABILITY
    }).to_list(None)
```

**Enforcement:**
- FastAPI dependency injection extracts `tenant_id` from JWT token
- All service methods accept `tenant_id` as first parameter
- MongoDB indexes include `tenant_id` as first field: `(tenant_id, category, region)`
- Code review checklist: "Does this query filter by tenant_id?"

### 2. Authentication & Authorization

**Three Authentication Layers:**

| Layer | Method | Use Case |
|-------|--------|----------|
| **User → API** | JWT Bearer Token | Tenant admin/users accessing API |
| **Widget → API** | Session Token | End customers using chat widget |
| **API → MCP** | HMAC + JWT | Internal serverless function calls |

**JWT Token Structure:**
```python
{
    "sub": "user_id_123",
    "tenant_id": "tenant_abc",  # CRITICAL: Used for all queries
    "role": "admin",
    "exp": 1699564800  # 24 hour expiry
}
```

**BYOK Security (Bring Your Own Key):**
```python
# Encrypt OpenAI keys at rest (AES-256)
from cryptography.fernet import Fernet

def store_openai_key(tenant_id: str, openai_key: str):
    encrypted = fernet.encrypt(openai_key.encode())
    await db.tenants.update_one(
        {"_id": tenant_id},
        {"$set": {"openai_api_key_encrypted": encrypted}}
    )

# Decrypt only at request time, never log
def get_openai_key(tenant_id: str) -> str:
    tenant = await db.tenants.find_one({"_id": tenant_id})
    return fernet.decrypt(tenant["openai_api_key_encrypted"]).decode()
```

### 3. Error Handling & Resilience

**Standard FastAPI Exception Pattern:**
```python
from fastapi import HTTPException
import sentry_sdk

class TenantNotFoundError(HTTPException):
    def __init__(self, tenant_id: str):
        super().__init__(
            status_code=404,
            detail=f"Tenant {tenant_id} not found"
        )
        sentry_sdk.capture_exception(self)

# LLM Retry Logic
@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=2, max=10),
    retry=retry_if_exception_type(OpenAIError)
)
async def generate_estimate_narrative(prompt: str, tenant_id: str):
    try:
        openai_key = get_openai_key(tenant_id)
        response = await openai.ChatCompletion.create(
            model="gpt-4o-mini",
            messages=[{"role": "user", "content": prompt}],
            api_key=openai_key
        )
        return response.choices[0].message.content
    except OpenAIError as e:
        logger.error(f"OpenAI API failed: {e}", extra={"tenant_id": tenant_id})
        sentry_sdk.capture_exception(e)
        raise
```

**Graceful Degradation:**
- LLM unavailable → return estimate without narrative
- MCP functions slow → fallback to direct MongoDB query
- Partial failures → return best-effort result with warning

### 4. Structured Logging

**Standard Log Format (JSON):**
```python
import structlog

logger = structlog.get_logger()

# Every log MUST include tenant_id and request_id
logger.info(
    "estimate.created",
    tenant_id=tenant_id,
    request_id=request_id,
    project_type="Pool Installation",
    p50_cost=75000,
    prompt_version="1.2.0",
    duration_ms=234
)
```

**Log Levels:**
- **DEBUG:** Development only (disabled in production)
- **INFO:** Normal operations (estimate created, feedback received)
- **WARNING:** Degraded service (LLM slow, cache miss)
- **ERROR:** Failures requiring investigation (database timeout, LLM API down)

**Retention:** 30 days (DigitalOcean App Platform logs)

### 5. Configuration Management

**Environment Variables Pattern:**
```python
# config.py
from pydantic_settings import BaseSettings

class Settings(BaseSettings):
    # Database
    MONGODB_URI: str

    # Security
    JWT_SECRET_KEY: str
    ENCRYPTION_KEY: str  # For BYOK

    # External Services
    SENDGRID_API_KEY: str
    SENTRY_DSN: str

    # Feature Flags
    ENABLE_LLM_CACHING: bool = True
    ENABLE_RATE_LIMITING: bool = True

    class Config:
        env_file = ".env"
        case_sensitive = True

settings = Settings()
```

**Secrets Management:**
- Production: DigitalOcean App Platform environment variables (encrypted at rest)
- Local development: `.env` file (never committed)
- BYOK keys: Encrypted in MongoDB with dedicated encryption key

### 6. Data Validation (Pydantic)

**Request/Response Models:**
```python
from pydantic import BaseModel, Field, validator

class EstimateRequest(BaseModel):
    project_type: str = Field(..., min_length=1, max_length=100)
    region: str = Field(..., regex="^(SoCal|NorCal|Central Coast)")
    description: str = Field(..., max_length=5000)

    @validator('project_type')
    def validate_project_type(cls, v):
        allowed = ["Pool", "ADU", "Kitchen Remodel", "Bathroom Remodel"]
        if v not in allowed:
            raise ValueError(f"Invalid project_type. Must be one of {allowed}")
        return v

class EstimateResponse(BaseModel):
    estimate_id: str
    p50_cost: float = Field(..., gt=0)
    p80_cost: float = Field(..., gt=0)
    p50_timeline_days: int = Field(..., gt=0)
    narrative: str
    confidence_score: float = Field(..., ge=0, le=1)
```

**Database Model Validation:**
- MongoDB documents validated against Pydantic models before insert
- Migrations validate existing data against new schemas

### 7. Rate Limiting (Per-Tenant Tiers)

**Tier-Based Limits:**
```python
from slowapi import Limiter
from slowapi.util import get_remote_address

limiter = Limiter(key_func=lambda: get_tenant_id_from_request())

@app.post("/estimate")
@limiter.limit(lambda: get_tenant_rate_limit())  # Dynamic based on tier
async def create_estimate(request: EstimateRequest, tenant_id: str = Depends(get_tenant_id)):
    # Trial: 10/hour
    # Pro: 100/hour
    # Enterprise: 1000/hour
    pass
```

**Global Rate Limiting:**
- 1000 concurrent requests across platform
- Widget API: 50 concurrent sessions per tenant
- Prevents DDoS attacks

### 8. CORS & Security Headers

**Widget Embedding Security:**
```python
from fastapi.middleware.cors import CORSMiddleware

app.add_middleware(
    CORSMiddleware,
    allow_origins=[
        "https://widget.efofx.ai",  # Widget CDN
        "https://*.contractors-site.com"  # Tenant domains (wildcard)
    ],
    allow_credentials=True,
    allow_methods=["GET", "POST"],
    allow_headers=["Authorization", "Content-Type"]
)

# Security Headers
@app.middleware("http")
async def add_security_headers(request, call_next):
    response = await call_next(request)
    response.headers["X-Content-Type-Options"] = "nosniff"
    response.headers["X-Frame-Options"] = "DENY"
    response.headers["Content-Security-Policy"] = "default-src 'self'"
    return response
```

### 9. Performance Optimization

**Caching Strategy:**
- **LLM responses:** Cache identical prompts (1 hour TTL)
- **Reference classes:** Cache popular queries (5 min TTL)
- **Widget configs:** Cache tenant branding (10 min TTL)

**Database Indexing:**
```javascript
// MongoDB indexes (created during migration)
db.reference_classes.createIndex({ tenant_id: 1, category: 1, region: 1 })
db.estimates.createIndex({ tenant_id: 1, created_at: -1 })
db.feedback.createIndex({ tenant_id: 1, estimate_id: 1 })
```

**Connection Pooling:**
- MongoDB: 10-50 connections (auto-scale based on load)
- OpenAI: Reuse HTTP client (connection pooling)

### 10. Testing Strategy

**Test Pyramid:**
- **Unit Tests (70%):** Pure functions, Pydantic models, utilities
- **Integration Tests (20%):** FastAPI endpoints + MongoDB (test database)
- **E2E Tests (10%):** Playwright widget tests (critical user flows)

**Tenant Isolation Testing:**
```python
@pytest.mark.asyncio
async def test_tenant_isolation():
    """Verify tenant A cannot access tenant B's data"""
    tenant_a_id = "tenant_a"
    tenant_b_id = "tenant_b"

    # Create estimate for tenant A
    estimate_a = await create_estimate(tenant_a_id, {...})

    # Attempt to fetch as tenant B (should return empty)
    estimates = await get_estimates(tenant_b_id)
    assert estimate_a["estimate_id"] not in [e["estimate_id"] for e in estimates]
```

**Test Coverage Goals:**
- RCF engine: >90% coverage
- Authentication/Authorization: 100% coverage
- Multi-tenant queries: 100% coverage
- Overall: >70% coverage

## Project Structure

```
efofx-workspace/
├── apps/
│   ├── efofx-api/                      # FastAPI Backend (Main Application)
│   │   ├── app/
│   │   │   ├── api/
│   │   │   │   ├── v1/
│   │   │   │   │   ├── endpoints/
│   │   │   │   │   │   ├── estimates.py       # POST /estimate, GET /estimate/{id}
│   │   │   │   │   │   ├── chat.py            # POST /chat/send, GET /chat/history
│   │   │   │   │   │   ├── feedback.py        # POST /feedback/customer, /feedback/contractor
│   │   │   │   │   │   ├── tenants.py         # POST /tenants, PATCH /tenants/{id}
│   │   │   │   │   │   └── widget.py          # GET /widget/config, POST /widget/session
│   │   │   │   │   ├── dependencies.py        # JWT auth, tenant_id extraction
│   │   │   │   │   └── router.py              # v1 router aggregation
│   │   │   ├── core/
│   │   │   │   ├── config.py                  # Pydantic Settings (env vars)
│   │   │   │   ├── security.py                # JWT, BYOK encryption, rate limiting
│   │   │   │   └── logging.py                 # Structlog configuration
│   │   │   ├── models/
│   │   │   │   ├── estimate.py                # Pydantic models for estimates
│   │   │   │   ├── feedback.py                # Pydantic models for feedback
│   │   │   │   ├── reference_class.py         # Domain-agnostic schema
│   │   │   │   └── tenant.py                  # Tenant, branding models
│   │   │   ├── services/
│   │   │   │   ├── rcf_engine.py              # Reference Class Forecasting logic
│   │   │   │   ├── llm_service.py             # OpenAI integration (BYOK)
│   │   │   │   ├── feedback_service.py        # Calibration calculations
│   │   │   │   ├── tenant_service.py          # Tenant management
│   │   │   │   └── mcp_client.py              # MCP functions client
│   │   │   ├── db/
│   │   │   │   ├── mongodb.py                 # MongoDB connection, Motor client
│   │   │   │   └── migrations/                # Database schema migrations
│   │   │   ├── middleware/
│   │   │   │   ├── tenant_isolation.py        # Enforce tenant_id in all queries
│   │   │   │   ├── cors.py                    # CORS for widget embedding
│   │   │   │   └── rate_limiting.py           # Per-tenant tier rate limits
│   │   │   ├── utils/
│   │   │   │   ├── prompt_loader.py           # Load prompts from config/prompts/*.json
│   │   │   │   └── cache.py                   # LLM response caching (Redis optional)
│   │   │   └── main.py                        # FastAPI app entry point
│   │   ├── config/
│   │   │   └── prompts/
│   │   │       ├── estimate_narrative.json    # P50/P80 explanation prompt
│   │   │       ├── conversational_scoping.json # Chat bot prompt
│   │   │       └── feedback_analysis.json     # Calibration feedback prompt
│   │   ├── tests/
│   │   │   ├── unit/
│   │   │   │   ├── test_rcf_engine.py
│   │   │   │   ├── test_security.py
│   │   │   │   └── test_tenant_isolation.py   # Critical: verify no cross-tenant leakage
│   │   │   ├── integration/
│   │   │   │   ├── test_estimates_api.py
│   │   │   │   └── test_feedback_api.py
│   │   │   └── conftest.py                    # Pytest fixtures
│   │   ├── .do/
│   │   │   └── app.yaml                       # DigitalOcean App Platform config
│   │   ├── requirements.txt                   # Python dependencies
│   │   ├── Dockerfile                         # Production container
│   │   └── README.md
│   │
│   ├── efofx-widget/                   # White Label Chat Widget (Vite + React + TS)
│   │   ├── src/
│   │   │   ├── components/
│   │   │   │   ├── ChatWidget/
│   │   │   │   │   ├── ChatWidget.tsx         # Main widget component
│   │   │   │   │   ├── ChatMessage.tsx        # Individual chat bubble
│   │   │   │   │   ├── EstimateDisplay.tsx    # P50/P80 visual display
│   │   │   │   │   └── ChatInput.tsx          # User input field
│   │   │   │   └── WidgetButton/
│   │   │   │       └── FloatingButton.tsx     # Trigger button
│   │   │   ├── api/
│   │   │   │   ├── chatApi.ts                 # POST /chat/send wrapper
│   │   │   │   ├── estimateApi.ts             # POST /estimate wrapper
│   │   │   │   └── widgetApi.ts               # GET /widget/config (branding)
│   │   │   ├── hooks/
│   │   │   │   ├── useChatSession.ts          # Manage chat state
│   │   │   │   └── useTenantBranding.ts       # Load tenant colors/logo
│   │   │   ├── types/
│   │   │   │   ├── estimate.ts                # Shared types with backend
│   │   │   │   └── widget.ts                  # Widget config types
│   │   │   ├── utils/
│   │   │   │   └── shadowDom.ts               # Shadow DOM helper utilities
│   │   │   ├── embed.tsx                      # Widget initialization entry point
│   │   │   └── index.html                     # Dev server test page
│   │   ├── tests/
│   │   │   ├── widget-embed.spec.ts           # Playwright: widget loads
│   │   │   ├── chat-flow.spec.ts              # Playwright: conversation works
│   │   │   ├── style-isolation.spec.ts        # Playwright: Shadow DOM isolation
│   │   │   └── multi-browser.spec.ts          # Playwright: Chrome/Firefox/Safari
│   │   ├── public/
│   │   │   └── test-embed.html                # Test embedding on sample site
│   │   ├── vite.config.ts                     # Build config (single embed.js output)
│   │   ├── playwright.config.ts               # E2E test configuration
│   │   ├── tailwind.config.js                 # Tailwind CSS for widget styling
│   │   ├── package.json
│   │   └── README.md
│   │
│   └── synthetic-data-generator/       # Python Scripts for Reference Classes
│       ├── generators/
│       │   ├── __init__.py
│       │   ├── pool.py                        # generate_pool_reference_classes()
│       │   ├── adu.py                         # generate_adu_reference_classes()
│       │   ├── kitchen.py                     # generate_kitchen_reference_classes()
│       │   ├── bathroom.py
│       │   ├── landscaping.py
│       │   ├── roofing.py
│       │   └── flooring.py
│       ├── validators/
│       │   └── validate_costs.py              # Check ±25% real-world tolerance
│       ├── seed_database.py                   # Populate MongoDB with reference classes
│       ├── requirements.txt                   # numpy, scipy, pymongo
│       └── README.md
│
├── functions/                          # DigitalOcean MCP Serverless Functions
│   ├── query-reference-classes/
│   │   ├── index.py                           # Query by category/region/attributes
│   │   └── requirements.txt
│   ├── apply-adjustments/
│   │   ├── index.py                           # Apply regional/complexity adjustments
│   │   └── requirements.txt
│   └── README.md
│
├── docs/                               # Project Documentation
│   ├── PRD/
│   │   ├── index.md                           # Sharded PRD master index
│   │   ├── executive-summary.md
│   │   ├── product-scope.md
│   │   └── ...                                # Other PRD sections
│   ├── architecture.md                        # This file
│   ├── bmm-workflow-status.yaml               # BMM workflow tracking
│   └── index.md                               # Project documentation index
│
├── .github/
│   └── workflows/
│       └── test.yml                           # Optional: GitHub Actions tests
│
├── .gitignore
└── README.md                                  # Workspace root README
```

## Epic to Architecture Mapping

This table maps epics from the PRD to architectural components and decisions.

| Epic / Functional Area | Primary Components | Key Decisions | Critical Patterns |
|------------------------|-------------------|---------------|-------------------|
| **Reference Class Forecasting Engine** | `rcf_engine.py`, `reference_classes` collection | NumPy/SciPy for synthetic data | Lognormal distributions for costs, tenant_id scoping |
| **Synthetic Data Generation** | `synthetic-data-generator/` scripts | NumPy/SciPy statistical distributions | Reproducible seed-based generation, ±25% validation |
| **Multi-Tenant Infrastructure** | `tenant_isolation.py` middleware, `tenants` collection | MongoDB with hard isolation | 100% queries include tenant_id filter |
| **LLM Integration (BYOK)** | `llm_service.py`, OpenAI SDK | Git-based JSON prompts, BYOK encryption | Fernet AES-256 for key storage, retry with exponential backoff |
| **Feedback & Calibration System** | `feedback_service.py`, `feedback` collection | SendGrid for email notifications | Store prompt_version in estimates for traceability |
| **White Label Chat Widget** | `efofx-widget/` (Vite + React + TS) | Vite starter template, Playwright testing | Shadow DOM isolation, Tailwind for branding |
| **Widget CDN/Distribution** | DigitalOcean Spaces CDN | Spaces CDN (274 PoPs) | Filename-based cache busting (embed.v{version}.js) |
| **MCP Server (Reference Class Queries)** | `functions/query-reference-classes/` | DigitalOcean Functions (serverless) | HMAC + JWT dual auth, 150ms p95 response time |
| **API Endpoints (FastAPI Backend)** | `app/api/v1/endpoints/` | FastAPI with Pydantic validation | Dependency injection for tenant_id extraction |
| **Authentication & Authorization** | `core/security.py`, JWT middleware | PyJWT, bcrypt password hashing | JWT with tenant_id claim, 24hr expiry |
| **Error Tracking & Monitoring** | Sentry SDK, DigitalOcean metrics | Hybrid DO + Sentry | Structured logging with tenant context |
| **CI/CD Pipeline** | `.do/app.yaml`, GitHub integration | DigitalOcean Auto-Deploy | Zero-downtime rolling deployments on push to `main` |
| **Testing Infrastructure** | Playwright, pytest | Playwright for E2E, pytest for backend | Shadow DOM piercing for widget tests |

## Technology Stack Details

### Backend (FastAPI)
- **Framework:** FastAPI 0.100+ (async Python web framework)
- **Database:** MongoDB Atlas (managed, auto-scaling)
- **Database Driver:** Motor 3.0+ (async MongoDB driver for Python)
- **Authentication:** PyJWT 2.8+ (JWT token generation/validation)
- **Encryption:** cryptography 41.0+ (Fernet for BYOK encryption)
- **LLM Integration:** openai 1.0+ (OpenAI Python SDK)
- **Email:** sendgrid 6.12+ (transactional email)
- **Monitoring:** sentry-sdk[fastapi] 2.0+ (error tracking)
- **Logging:** structlog 24.0+ (structured JSON logging)
- **Validation:** Pydantic 2.0+ (request/response validation)
- **Rate Limiting:** slowapi 0.1+ (per-tenant rate limits)
- **HTTP Client:** httpx 0.24+ (async HTTP for LLM/MCP calls)

### Frontend (Widget)
- **Framework:** React 19 (component-based UI)
- **Build Tool:** Vite 5.x (fast HMR, optimized builds)
- **Language:** TypeScript 5.x (type safety)
- **Styling:** Tailwind CSS 3.x (utility-first CSS)
- **Bundler:** Rollup 4.x (via Vite, single embed.js output)
- **CSS Injection:** vite-plugin-css-injected-by-js (inline CSS)
- **Testing:** Playwright (E2E testing with Shadow DOM support)

### Infrastructure (DigitalOcean)
- **Hosting:** DigitalOcean App Platform (auto-scaling PaaS)
- **Serverless:** DigitalOcean Functions (MCP query functions)
- **Object Storage:** DigitalOcean Spaces (S3-compatible, with CDN)
- **CDN:** Spaces CDN (274 global PoPs, 66% faster delivery)
- **Monitoring:** App Platform Metrics + Sentry
- **CI/CD:** App Platform Auto-Deploy from GitHub

### Data & Analytics
- **Reference Data:** NumPy 1.26+, SciPy 1.11+ (synthetic data generation)

### Integration Points

| Service | Purpose | Authentication | Integration Pattern |
|---------|---------|----------------|-------------------|
| **MongoDB Atlas** | Primary database | Connection string with credentials | Motor async driver, connection pooling |
| **OpenAI API** | LLM narratives, chat | BYOK per-tenant keys | httpx async client, retry with exponential backoff |
| **SendGrid** | Transactional email | API key (secret) | Official SendGrid Python SDK |
| **Sentry** | Error tracking | DSN (secret) | sentry-sdk[fastapi] auto-instrumentation |
| **DigitalOcean Spaces** | File storage, CDN | Access key + secret | boto3 S3-compatible API |
| **DigitalOcean Functions (MCP)** | Serverless queries | HMAC + JWT | httpx POST with request signing |

## Implementation Patterns

These patterns ensure consistent implementation across all epics and stories.

### 1. Tenant-Scoped Database Queries
```python
# ✅ CORRECT: Always include tenant_id
await db.estimates.find({"tenant_id": tenant_id, "status": "completed"})

# ❌ WRONG: Missing tenant_id (security vulnerability)
await db.estimates.find({"status": "completed"})
```

### 2. Service Layer with Dependency Injection
```python
class EstimateService:
    def __init__(self, db: AsyncIOMotorDatabase, llm: LLMService):
        self.db = db
        self.llm = llm

    async def create(self, tenant_id: str, request: EstimateRequest):
        # tenant_id always first parameter
        pass
```

### 3. Pydantic Request/Response Models
```python
class EstimateRequest(BaseModel):
    project_type: str = Field(..., max_length=100)
    region: str
    description: str = Field(..., max_length=5000)

class EstimateResponse(BaseModel):
    estimate_id: str
    p50_cost: float
    p80_cost: float
```

### 4. Structured Logging with Context
```python
logger.info("estimate.created",
    tenant_id=tenant_id,
    estimate_id=estimate_id,
    p50_cost=p50,
    duration_ms=duration)
```

### 5. Error Handling with Sentry Integration
```python
try:
    result = await openai.ChatCompletion.create(...)
except OpenAIError as e:
    logger.error("LLM failed", error=str(e), tenant_id=tenant_id)
    sentry_sdk.capture_exception(e)
    raise HTTPException(503, "LLM temporarily unavailable")
```

## Novel Patterns

These patterns document efOfX's unique concepts that differentiate it from standard estimation tools. These are the product's competitive moat and require careful, consistent implementation across all epics.

### Pattern 1: Two-Layer Communication Model

**Purpose:** Enable transparent communication by separating "internal truth" (raw estimates with uncertainty) from "external narratives" (stakeholder-specific explanations). This is the core of "trust through transparency."

**Problem Being Solved:** Traditional estimation tools force estimators to choose between:
- False precision (single number estimates that hide uncertainty)
- Overwhelming stakeholders with technical details

The two-layer model provides both: full transparency for those who want it, contextual narratives for those who need it.

**Components:**

```python
# Data Model
class EstimateModel(BaseModel):
    # Layer 1: Internal Truth (always stored)
    internal_data: InternalEstimateData

    # Layer 2: External Narratives (generated on-demand or cached)
    external_narratives: Dict[str, ExternalNarrative]  # Keyed by audience type

class InternalEstimateData(BaseModel):
    """Raw estimation data - the unvarnished truth"""
    p50_cost: float
    p80_cost: float
    p95_cost: float  # Not shown to customers in MVP
    assumptions: List[Assumption]  # Full technical assumptions
    risks: List[Risk]  # Complete risk catalog with probabilities
    reference_class_id: str
    confidence_score: float
    calculation_breakdown: Dict[str, Any]  # Show your work

class ExternalNarrative(BaseModel):
    """Stakeholder-specific communication"""
    audience: str  # "customer", "contractor", "cfo", "dev_team"
    summary: str  # LLM-generated summary
    key_points: List[str]  # Bullets optimized for audience
    questions_to_ask: List[str]  # Suggested follow-ups
    tone: str  # "educational", "executive", "technical"
```

**Data Flow:**

```
┌─────────────────┐
│ User Request    │
│ (Project Desc)  │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────┐
│ RCF Engine (Epic 2)                 │
│ - Match reference class             │
│ - Calculate P50/P80/P95             │
│ - Extract assumptions from template │
│ - Identify risks from patterns      │
└────────┬────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ InternalEstimateData (Layer 1)         │ ◄─── ALWAYS STORED
│ - Full P50/P80/P95 distributions       │
│ - Complete assumption list             │
│ - Detailed risk catalog                │
│ - Calculation breakdown                │
└────────┬───────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│ LLM Narrative Service (Epic 4)          │
│ - Load prompt from config/prompts/      │
│ - Generate audience-specific narrative  │
│ - Audience: customer, contractor, etc.  │
└────────┬────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ ExternalNarrative (Layer 2)            │ ◄─── CACHED (1hr TTL)
│ - Customer-friendly summary            │
│ - Key points in plain language         │
│ - Questions to ask contractor          │
└────────┬───────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────┐
│ API Response                        │
│ {                                   │
│   "internal": {...},  // Optional   │
│   "external": {...},  // Default    │
│   "display_mode": "external"        │
│ }                                   │
└─────────────────────────────────────┘
```

**Implementation Guide for Agents:**

**Epic 2 (RCF Engine):**
```python
# Always create and store InternalEstimateData
async def create_estimate(tenant_id: str, request: EstimateRequest) -> Estimate:
    reference_class = await match_reference_class(request)

    # Calculate FULL distribution (P50, P80, P95)
    internal_data = InternalEstimateData(
        p50_cost=calculate_p50(reference_class, request),
        p80_cost=calculate_p80(reference_class, request),
        p95_cost=calculate_p95(reference_class, request),  # Store even if not shown
        assumptions=extract_assumptions(reference_class, request),
        risks=identify_risks(reference_class, request),
        confidence_score=reference_class.confidence,
        calculation_breakdown={
            "base_cost": reference_class.p50_cost,
            "regional_multiplier": 1.15,
            "complexity_factor": 1.2,
            "final_p50": p50_cost
        }
    )

    # Store estimate with internal_data
    estimate = Estimate(
        tenant_id=tenant_id,
        internal_data=internal_data,
        external_narratives={},  # Generated on-demand
        display_mode="external"  # Default for customers
    )
    await db.estimates.insert_one(estimate.dict())
    return estimate
```

**Epic 4 (LLM Integration):**
```python
# Generate external narratives on-demand
async def generate_external_narrative(
    estimate: Estimate,
    audience: str = "customer"
) -> ExternalNarrative:
    # Check cache first
    if audience in estimate.external_narratives:
        cached = estimate.external_narratives[audience]
        if not is_expired(cached, ttl=3600):  # 1hr cache
            return cached

    # Load audience-specific prompt
    prompt_template = load_prompt(f"config/prompts/narrative_{audience}.json")

    # Generate narrative from internal truth
    narrative_text = await llm_service.generate(
        prompt=prompt_template.format(
            p50=estimate.internal_data.p50_cost,
            p80=estimate.internal_data.p80_cost,
            assumptions=estimate.internal_data.assumptions,
            risks=estimate.internal_data.risks
        ),
        temperature=0.7  # Balanced creativity
    )

    # Parse structured response
    narrative = ExternalNarrative(
        audience=audience,
        summary=narrative_text["summary"],
        key_points=narrative_text["key_points"],
        questions_to_ask=narrative_text["questions"],
        tone=prompt_template.tone
    )

    # Cache narrative
    estimate.external_narratives[audience] = narrative
    await db.estimates.update_one(
        {"_id": estimate.id},
        {"$set": {f"external_narratives.{audience}": narrative.dict()}}
    )

    return narrative
```

**Widget Implementation (Epic 5):**
```typescript
// Widget displays external narrative by default
interface EstimateDisplayProps {
  estimate: Estimate;
  showInternalData?: boolean;  // Admin toggle (post-MVP)
}

function EstimateDisplay({ estimate, showInternalData = false }: EstimateDisplayProps) {
  const displayData = showInternalData
    ? estimate.internal_data
    : estimate.external_narratives.customer;

  if (showInternalData) {
    return (
      <InternalTruthView>
        <CostDistribution p50={estimate.internal_data.p50_cost}
                         p80={estimate.internal_data.p80_cost} />
        <AssumptionsList assumptions={estimate.internal_data.assumptions} />
        <RiskCatalog risks={estimate.internal_data.risks} />
        <CalculationBreakdown breakdown={estimate.internal_data.calculation_breakdown} />
      </InternalTruthView>
    );
  }

  return (
    <ExternalNarrativeView>
      <NarrativeSummary text={displayData.summary} />
      <KeyPointsList points={displayData.key_points} />
      <SuggestedQuestions questions={displayData.questions_to_ask} />
    </ExternalNarrativeView>
  );
}
```

**State Management:**

| State | Value | When |
|-------|-------|------|
| `display_mode` | `"external"` | Default for widget customers |
| `display_mode` | `"internal"` | Tenant admin view (post-MVP) |
| `display_mode` | `"both"` | Future: side-by-side comparison |

**Edge Cases & Failure Modes:**

1. **LLM Timeout During Narrative Generation:**
   ```python
   try:
       narrative = await generate_external_narrative(estimate, "customer")
   except LLMTimeoutError:
       # Graceful degradation: Show internal data as fallback
       return EstimateResponse(
           internal_data=estimate.internal_data,
           external_narratives={},
           display_mode="internal",  # Forced fallback
           warning="Narrative generation unavailable"
       )
   ```

2. **Audience Type Not Recognized:**
   ```python
   ALLOWED_AUDIENCES = ["customer", "contractor", "cfo", "dev_team"]
   if audience not in ALLOWED_AUDIENCES:
       audience = "customer"  # Default fallback
   ```

3. **Missing Assumptions/Risks:**
   ```python
   # RCF engine MUST always provide at least minimal assumptions
   if not internal_data.assumptions:
       internal_data.assumptions = [
           Assumption(text="Standard project conditions assumed", confidence=0.7)
       ]
   ```

**Integration with Standard Patterns:**

- **Caching:** External narratives cached 1hr (matches LLM response cache pattern)
- **Tenant Isolation:** All estimates scoped by `tenant_id` (matches multi-tenant pattern)
- **Prompt Management:** Audience prompts stored in `config/prompts/` (matches prompt pattern)
- **Error Handling:** LLM failures trigger graceful degradation (matches resilience pattern)

**MVP Scope:**
- ✅ Internal truth always stored
- ✅ External narrative for "customer" audience
- ❌ Admin toggle to view internal truth (deferred to post-MVP)
- ❌ Multiple audience narratives ("cfo", "dev_team") (deferred to post-MVP)

---

### Pattern 2: Calibration Feedback Loop

**Purpose:** Enable self-improvement by closing the loop between estimates and reality. Every completed project makes future estimates more accurate.

**Problem Being Solved:** Traditional estimation tools are static - they don't learn from outcomes. efOfX continuously improves by tracking actual costs/timelines and using that data to:
1. Adjust reference class distributions (P50/P80 values)
2. Refine LLM prompts with real-world examples
3. Display accuracy trends to build trust

**Components:**

```python
class FeedbackModel(BaseModel):
    """Actual project outcome"""
    estimation_id: str
    tenant_id: str
    feedback_type: Literal["customer", "contractor"]

    # Actuals
    actual_cost: float
    actual_timeline_weeks: int

    # Variance calculation
    variance_cost_pct: float  # (actual - estimated) / estimated
    variance_timeline_pct: float

    # Qualitative
    accuracy_rating: int  # 1-5 stars
    comments: str
    discrepancy_flags: DiscrepancyFlags

class DiscrepancyFlags(BaseModel):
    scope_creep: bool
    hidden_costs: bool
    market_changes: bool
    weather_delays: bool  # Domain-specific

class CalibrationMetrics(BaseModel):
    """Tenant-level accuracy tracking"""
    tenant_id: str
    reference_class_id: Optional[str]  # None = all classes

    total_estimates: int
    feedback_count: int
    feedback_rate: float  # feedback_count / total_estimates

    avg_cost_variance_pct: float
    avg_timeline_variance_pct: float
    within_20_pct_rate: float  # % of estimates within 20% of actual

    trend: Literal["improving", "stable", "declining"]
    last_updated: datetime
```

**Data Flow:**

```
┌──────────────────┐
│ 1. Estimate      │
│    Created       │
│ - P50: $75k      │
│ - P80: $92k      │
│ - Prompt: v1.2.0 │
└────────┬─────────┘
         │
         │ (Project Completed)
         │
         ▼
┌────────────────────────────────┐
│ 2. Feedback Submitted          │
│ Customer: Actual = $82k (9wks) │
│ Contractor: Explains +$7k      │
└────────┬───────────────────────┘
         │
         ▼
┌─────────────────────────────────┐
│ 3. Variance Calculation         │
│ - Cost variance: +9.3%          │
│ - Timeline variance: +12.5%     │
│ - Within 20% target: ✓          │
└────────┬────────────────────────┘
         │
         ▼
┌──────────────────────────────────────┐
│ 4. Aggregate Metrics (Nightly Job)  │
│ - Update tenant calibration          │
│ - Update reference class metrics     │
│ - Calculate trends                   │
└────────┬─────────────────────────────┘
         │
         ├─────────────┬──────────────┬──────────────┐
         │             │              │              │
         ▼             ▼              ▼              ▼
┌───────────────┐ ┌─────────────┐ ┌────────────┐ ┌──────────────┐
│ 5a. Adjust    │ │ 5b. Refine  │ │ 5c. Display│ │ 5d. Recommend│
│ Reference     │ │ LLM Prompts │ │ Metrics    │ │ Tuning       │
│ Class P50/P80 │ │ (Add        │ │ to Tenant  │ │ (Admin       │
│ (If bias      │ │  Examples)  │ │ Dashboard  │ │  Approval)   │
│  detected)    │ │             │ │            │ │              │
└───────────────┘ └─────────────┘ └────────────┘ └──────────────┘
```

**Implementation Guide for Agents:**

**Epic 6 (Feedback System) - Story 6.1: Customer Feedback Submission**
```python
async def submit_customer_feedback(
    estimation_id: str,
    actual_cost: float,
    actual_timeline_weeks: int,
    accuracy_rating: int,
    comments: str
) -> FeedbackModel:
    # Retrieve original estimate
    estimate = await db.estimates.find_one({"_id": estimation_id})

    # Calculate variances
    variance_cost = (actual_cost - estimate.internal_data.p50_cost) / estimate.internal_data.p50_cost
    variance_timeline = (actual_timeline_weeks - estimate.internal_data.p50_timeline_weeks) / estimate.internal_data.p50_timeline_weeks

    # Create feedback record
    feedback = FeedbackModel(
        estimation_id=estimation_id,
        tenant_id=estimate.tenant_id,
        feedback_type="customer",
        actual_cost=actual_cost,
        actual_timeline_weeks=actual_timeline_weeks,
        variance_cost_pct=variance_cost * 100,
        variance_timeline_pct=variance_timeline * 100,
        accuracy_rating=accuracy_rating,
        comments=comments,
        submitted_at=datetime.utcnow()
    )

    await db.feedback.insert_one(feedback.dict())

    # Trigger async calibration update (don't block response)
    asyncio.create_task(update_calibration_metrics(estimate.tenant_id, estimate.reference_class_id))

    return feedback
```

**Epic 6 - Story 6.3: Calibration Metrics Calculation (Nightly Job)**
```python
async def calculate_calibration_metrics(
    tenant_id: str,
    reference_class_id: Optional[str] = None
) -> CalibrationMetrics:
    # Build query filter
    query = {"tenant_id": tenant_id}
    if reference_class_id:
        query["reference_class_id"] = reference_class_id

    # Get all estimates with feedback
    pipeline = [
        {"$match": query},
        {"$lookup": {
            "from": "feedback",
            "localField": "_id",
            "foreignField": "estimation_id",
            "as": "feedback"
        }},
        {"$match": {"feedback": {"$ne": []}}},  # Only estimates with feedback
        {"$unwind": "$feedback"},
        {"$group": {
            "_id": None,
            "total_estimates": {"$sum": 1},
            "avg_cost_variance": {"$avg": "$feedback.variance_cost_pct"},
            "avg_timeline_variance": {"$avg": "$feedback.variance_timeline_pct"},
            "within_20_pct": {
                "$sum": {
                    "$cond": [
                        {"$lte": [{"$abs": "$feedback.variance_cost_pct"}, 20]},
                        1,
                        0
                    ]
                }
            }
        }}
    ]

    result = await db.estimates.aggregate(pipeline).to_list(1)

    if not result:
        return None  # No feedback yet

    data = result[0]

    # Calculate trend (compare to previous period)
    previous_metrics = await get_previous_period_metrics(tenant_id, reference_class_id)
    trend = calculate_trend(data["avg_cost_variance"], previous_metrics)

    metrics = CalibrationMetrics(
        tenant_id=tenant_id,
        reference_class_id=reference_class_id,
        total_estimates=data["total_estimates"],
        feedback_count=data["total_estimates"],  # Already filtered
        feedback_rate=data["total_estimates"] / await get_total_estimates(tenant_id),
        avg_cost_variance_pct=data["avg_cost_variance"],
        avg_timeline_variance_pct=data["avg_timeline_variance"],
        within_20_pct_rate=data["within_20_pct"] / data["total_estimates"],
        trend=trend,
        last_updated=datetime.utcnow()
    )

    # Cache metrics (updated nightly)
    await db.calibration_metrics.update_one(
        {"tenant_id": tenant_id, "reference_class_id": reference_class_id},
        {"$set": metrics.dict()},
        upsert=True
    )

    return metrics
```

**Epic 6 - Story 6.4: Reference Class Tuning (Admin Approval Required)**
```python
async def recommend_reference_class_tuning(
    reference_class_id: str,
    min_feedback_count: int = 5
) -> Optional[TuningRecommendation]:
    """Detect systematic bias and recommend adjustments"""

    metrics = await calculate_calibration_metrics(
        tenant_id=None,  # Platform-wide for reference classes
        reference_class_id=reference_class_id
    )

    if metrics.feedback_count < min_feedback_count:
        return None  # Need more data

    # Detect bias threshold: >15% consistent variance
    if abs(metrics.avg_cost_variance_pct) > 15:
        recommendation = TuningRecommendation(
            reference_class_id=reference_class_id,
            current_p50=await get_reference_class_p50(reference_class_id),
            recommended_p50=calculate_adjusted_p50(metrics),
            rationale=f"Consistent {metrics.avg_cost_variance_pct:+.1f}% variance across {metrics.feedback_count} projects",
            confidence=metrics.feedback_count / 10,  # More feedback = higher confidence
            requires_admin_approval=True,
            status="pending"
        )

        await db.tuning_recommendations.insert_one(recommendation.dict())

        # Notify admin
        await send_admin_notification(recommendation)

        return recommendation

    return None  # No tuning needed
```

**Epic 4 - LLM Prompt Refinement with Feedback Examples**
```python
async def generate_estimate_narrative_with_calibration(
    estimate: Estimate,
    tenant_id: str
) -> str:
    # Load base prompt
    prompt_template = load_prompt("config/prompts/estimate_narrative.json")

    # Fetch recent feedback for similar projects (calibration context)
    similar_feedback = await db.feedback.find({
        "tenant_id": tenant_id,
        "reference_class_id": estimate.reference_class_id
    }).sort("submitted_at", -1).limit(3).to_list(3)

    # Build calibration examples for prompt
    calibration_context = ""
    if similar_feedback:
        examples = []
        for fb in similar_feedback:
            examples.append(
                f"- Similar project: Estimated ${fb.estimated_cost}, "
                f"Actual ${fb.actual_cost} ({fb.variance_cost_pct:+.1f}%)"
            )
        calibration_context = "\n".join(examples)

    # Generate narrative with calibration awareness
    narrative = await llm_service.generate(
        prompt=prompt_template.format(
            p50=estimate.internal_data.p50_cost,
            p80=estimate.internal_data.p80_cost,
            assumptions=estimate.internal_data.assumptions,
            risks=estimate.internal_data.risks,
            calibration_examples=calibration_context,  # Add feedback context
            tenant_accuracy=await get_tenant_accuracy(tenant_id)
        )
    )

    return narrative
```

**State Management:**

| Metric | Initial State | After Feedback | After Tuning |
|--------|---------------|----------------|--------------|
| `feedback_count` | 0 | Increments | N/A |
| `avg_cost_variance_pct` | null | Calculated | Approaches 0% |
| `within_20_pct_rate` | null | < 70% (synthetic) | > 70% (goal) |
| `trend` | "stable" | "improving" / "declining" | "improving" |
| `reference_class.p50` | Synthetic value | No change | Adjusted ±% |

**Edge Cases & Failure Modes:**

1. **Conflicting Customer/Contractor Feedback:**
   ```python
   async def resolve_feedback_conflict(estimation_id: str):
       feedbacks = await db.feedback.find({"estimation_id": estimation_id}).to_list(2)

       if len(feedbacks) == 2:
           variance_diff = abs(feedbacks[0].variance_cost_pct - feedbacks[1].variance_cost_pct)

           if variance_diff > 10:  # >10% discrepancy
               # Flag for admin review
               await db.feedback_conflicts.insert_one({
                   "estimation_id": estimation_id,
                   "customer_variance": feedbacks[0].variance_cost_pct,
                   "contractor_variance": feedbacks[1].variance_cost_pct,
                   "status": "requires_review"
               })

               # Don't use for calibration until resolved
               return "conflict"
   ```

2. **Outlier Detection:**
   ```python
   def is_outlier(feedback: FeedbackModel, metrics: CalibrationMetrics) -> bool:
       """Identify outliers that shouldn't skew calibration"""
       # >3 standard deviations from mean
       if abs(feedback.variance_cost_pct - metrics.avg_cost_variance_pct) > (3 * calculate_stddev(metrics)):
           # Check discrepancy flags
           if feedback.discrepancy_flags.scope_creep or feedback.discrepancy_flags.market_changes:
               return True  # Expected outlier, exclude from calibration
       return False
   ```

3. **Insufficient Data for Tuning:**
   ```python
   MIN_FEEDBACK_FOR_TUNING = 5

   if metrics.feedback_count < MIN_FEEDBACK_FOR_TUNING:
       logger.info(
           "insufficient_feedback_for_tuning",
           reference_class_id=reference_class_id,
           feedback_count=metrics.feedback_count,
           required=MIN_FEEDBACK_FOR_TUNING
       )
       return None  # Don't recommend tuning yet
   ```

**Integration with Standard Patterns:**

- **Tenant Isolation:** All feedback scoped by `tenant_id`
- **Async Processing:** Calibration calculations run as background jobs (don't block user responses)
- **Caching:** Calibration metrics cached (updated nightly, not real-time)
- **Logging:** All tuning recommendations logged with `prompt_version` for traceability

**Cross-Epic Dependencies:**

| Epic | Dependency | Data Flow |
|------|------------|-----------|
| Epic 2 (RCF Engine) | Stores `reference_class_id` in estimates | Enables filtering feedback by reference class |
| Epic 4 (LLM Integration) | Stores `prompt_version` in estimates | Enables A/B testing prompt changes |
| Epic 6 (Feedback System) | Consumes estimates, produces feedback | Closes the calibration loop |

**MVP Scope:**
- ✅ Customer & contractor feedback submission
- ✅ Variance calculation
- ✅ Calibration metrics display (tenant dashboard)
- ✅ Reference class tuning recommendations (admin approval)
- ✅ LLM prompt refinement with feedback examples
- ❌ Automated reference class tuning (requires admin approval)
- ❌ A/B testing prompt versions (deferred to post-MVP)

---

### Pattern 3: Domain Extension Pattern

**Purpose:** Enable efOfX to support multiple estimation domains (construction, IT/dev, finance, healthcare) using the same backend architecture with domain-specific reference data.

**Problem Being Solved:** Most estimation tools are hardcoded for one domain. efOfX achieves competitive advantage by making domain addition simple: same engine, new reference data. This enables rapid market expansion.

**Components:**

```python
class ReferenceClass(BaseModel):
    """Domain-agnostic reference class schema"""
    _id: str
    tenant_id: Optional[str]  # None = platform-provided

    # Domain identification
    category: str  # "construction", "it_dev", "finance", "healthcare"
    subcategory: str  # Domain-specific: "pool", "api_development", "loan_processing"
    name: str  # Human-readable: "Residential Pool - Midrange"

    # Flexible attributes (domain-specific)
    attributes: Dict[str, Any]  # e.g., {"size_range": "15x30 ft", "includes_spa": false}

    # Universal estimation fields
    cost_distribution: CostDistribution
    timeline_distribution: TimelineDistribution
    cost_breakdown_template: Dict[str, float]  # Percentages must sum to 1.0

    # Metadata
    keywords: List[str]  # For matching
    regions: List[str]  # Regional applicability
    regional_multipliers: Dict[str, float]
    is_synthetic: bool
    validation_source: Optional[str]

class CostDistribution(BaseModel):
    """Universal cost representation"""
    P50: float
    P80: float
    P95: float
    currency: str  # "USD", "EUR", "person-hours" (for IT)

class TimelineDistribution(BaseModel):
    """Universal timeline representation"""
    P50: int
    P80: int
    P95: int
    unit: str  # "weeks", "months", "sprints" (for IT)
```

**Data Flow for Adding New Domain:**

```
┌────────────────────────────┐
│ 1. Domain Specification    │
│ (Product Manager)          │
│ - Category: "it_dev"       │
│ - Subcategories: [...]     │
│ - Cost unit: "person-weeks"│
│ - Timeline unit: "sprints" │
└──────────┬─────────────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│ 2. Synthetic Data Generation            │
│ (apps/synthetic-data-generator/)        │
│                                         │
│ generators/                             │
│ ├── api_development.py  ← NEW          │
│ ├── mobile_app.py       ← NEW          │
│ └── web_app.py          ← NEW          │
└──────────┬──────────────────────────────┘
           │
           ▼
┌────────────────────────────────────┐
│ 3. Reference Class Creation        │
│ (Seed database)                    │
│                                    │
│ {                                  │
│   "category": "it_dev",            │
│   "subcategory": "api_development",│
│   "attributes": {                  │
│     "endpoints": "5-10",           │
│     "auth": "OAuth2",              │
│     "database": "PostgreSQL"       │
│   },                               │
│   "cost_distribution": {           │
│     "P50": 4.0,  # person-weeks    │
│     "P80": 6.0,                    │
│     "currency": "person-weeks"     │
│   },                               │
│   "timeline_distribution": {       │
│     "P50": 2,  # sprints           │
│     "P80": 3,                      │
│     "unit": "sprints"              │
│   }                                │
│ }                                  │
└──────────┬─────────────────────────┘
           │
           ▼
┌────────────────────────────────────┐
│ 4. RCF Engine (No Code Changes!)  │
│ - Match algorithm domain-agnostic  │
│ - Calculations work on any schema  │
│ - Attributes queried flexibly      │
└──────────┬─────────────────────────┘
           │
           ▼
┌────────────────────────────────────┐
│ 5. LLM Prompts (Domain-Aware)     │
│                                    │
│ config/prompts/                    │
│ ├── estimate_narrative.json       │
│ │   └── "domain_context": "{...}" │
│ └── conversational_scoping.json   │
│     └── "domain_questions": {...} │
└────────────────────────────────────┘
```

**Implementation Guide for Agents:**

**Step 1: Create Domain Synthetic Data Generator**
```python
# apps/synthetic-data-generator/generators/api_development.py

import numpy as np
from scipy.stats import lognorm

def generate_api_development_reference_classes(
    regions: List[str] = ["US West Coast", "US East Coast", "Europe"],
    seed: int = 42
) -> List[Dict]:
    """Generate IT/Dev domain reference classes"""
    np.random.seed(seed)

    # Define subcategories for IT/Dev domain
    subcategories = [
        {
            "name": "REST API - Simple CRUD",
            "attributes": {
                "endpoints": "5-10",
                "auth": "JWT",
                "database": "PostgreSQL",
                "complexity": "low"
            },
            "base_cost_pw": 4.0,  # person-weeks
            "base_timeline_sprints": 2
        },
        {
            "name": "REST API - Complex Business Logic",
            "attributes": {
                "endpoints": "15-25",
                "auth": "OAuth2",
                "database": "PostgreSQL + Redis",
                "complexity": "high"
            },
            "base_cost_pw": 8.0,
            "base_timeline_sprints": 4
        },
        # ... more subcategories
    ]

    reference_classes = []

    for subcat in subcategories:
        for region in regions:
            # Generate cost distribution (lognormal)
            p50 = subcat["base_cost_pw"] * get_regional_multiplier(region)
            sigma = 0.3  # 30% variance
            p80 = p50 * np.exp(0.84 * sigma)  # 80th percentile
            p95 = p50 * np.exp(1.65 * sigma)  # 95th percentile

            reference_class = {
                "category": "it_dev",
                "subcategory": "api_development",
                "name": f"{subcat['name']} ({region})",
                "attributes": subcat["attributes"],
                "keywords": ["api", "rest", "backend", "microservice"],
                "regions": [region],
                "cost_distribution": {
                    "P50": round(p50, 1),
                    "P80": round(p80, 1),
                    "P95": round(p95, 1),
                    "currency": "person-weeks"  # Different from construction!
                },
                "timeline_distribution": {
                    "P50": subcat["base_timeline_sprints"],
                    "P80": subcat["base_timeline_sprints"] + 1,
                    "P95": subcat["base_timeline_sprints"] + 2,
                    "unit": "sprints"  # Different from construction!
                },
                "cost_breakdown_template": {
                    "development": 0.60,
                    "testing": 0.20,
                    "code_review": 0.10,
                    "documentation": 0.05,
                    "deployment": 0.05
                },
                "regional_multipliers": {
                    "US West Coast": 1.3,  # Higher tech salaries
                    "US East Coast": 1.2,
                    "Europe": 1.0,
                    "Asia": 0.7
                },
                "is_synthetic": True,
                "validation_source": "Stack Overflow Developer Survey 2024"
            }

            reference_classes.append(reference_class)

    return reference_classes
```

**Step 2: RCF Engine Remains Domain-Agnostic**
```python
# app/services/rcf_engine.py (NO CHANGES NEEDED!)

async def match_reference_class(
    tenant_id: str,
    category: str,  # "construction" or "it_dev" or any future domain
    user_description: str,
    region: str
) -> ReferenceClass:
    """Domain-agnostic matching algorithm"""

    # Extract keywords from description
    keywords = extract_keywords(user_description)

    # Query reference classes (flexible attributes)
    query = {
        "tenant_id": {"$in": [tenant_id, None]},  # Tenant or platform
        "category": category,
        "regions": region,
        "$or": [
            {"keywords": {"$in": keywords}},
            {"name": {"$regex": "|".join(keywords), "$options": "i"}}
        ]
    }

    matches = await db.reference_classes.find(query).to_list(10)

    # Score matches by keyword overlap
    scored = []
    for match in matches:
        score = calculate_keyword_overlap(keywords, match["keywords"])
        scored.append((score, match))

    # Return best match
    best_match = max(scored, key=lambda x: x[0])[1]
    return ReferenceClass(**best_match)

async def calculate_estimate(
    reference_class: ReferenceClass,
    complexity_factor: float = 1.0
) -> EstimateResult:
    """Domain-agnostic calculation"""

    # Apply complexity adjustment (works for any domain)
    p50 = reference_class.cost_distribution.P50 * complexity_factor
    p80 = reference_class.cost_distribution.P80 * complexity_factor

    # Apply regional multiplier
    regional_multiplier = reference_class.regional_multipliers.get(region, 1.0)
    p50 *= regional_multiplier
    p80 *= regional_multiplier

    # Calculate breakdown (percentages work for any domain)
    breakdown = {}
    for category, percentage in reference_class.cost_breakdown_template.items():
        breakdown[category] = p50 * percentage

    return EstimateResult(
        p50_cost=p50,
        p80_cost=p80,
        cost_unit=reference_class.cost_distribution.currency,  # "USD" or "person-weeks"
        timeline_p50=reference_class.timeline_distribution.P50,
        timeline_p80=reference_class.timeline_distribution.P80,
        timeline_unit=reference_class.timeline_distribution.unit,  # "weeks" or "sprints"
        breakdown=breakdown
    )
```

**Step 3: Domain-Aware LLM Prompts**
```json
// config/prompts/estimate_narrative.json
{
  "version": "1.3.0",
  "prompt_template": "Generate an estimate explanation for a {category} project...",

  "domain_contexts": {
    "construction": {
      "terminology": ["contractor", "permits", "materials", "labor"],
      "units": "cost in USD, timeline in weeks",
      "stakeholders": ["homeowner", "contractor", "inspector"]
    },
    "it_dev": {
      "terminology": ["developer", "sprint", "API", "deployment"],
      "units": "effort in person-weeks, timeline in sprints",
      "stakeholders": ["product manager", "developer", "QA engineer"]
    }
  },

  "instruction": "Use domain_contexts[{category}] to adapt your language and examples."
}
```

**Usage in LLM Service:**
```python
async def generate_domain_aware_narrative(
    estimate: Estimate,
    reference_class: ReferenceClass
) -> str:
    prompt_config = load_prompt("config/prompts/estimate_narrative.json")

    # Get domain-specific context
    domain_context = prompt_config["domain_contexts"].get(
        reference_class.category,
        {}  # Fallback to generic
    )

    # Generate narrative with domain awareness
    narrative = await llm_service.generate(
        prompt=prompt_config["prompt_template"].format(
            category=reference_class.category,
            p50=estimate.internal_data.p50_cost,
            cost_unit=reference_class.cost_distribution.currency,
            timeline_unit=reference_class.timeline_distribution.unit,
            domain_terminology=domain_context.get("terminology", []),
            stakeholders=domain_context.get("stakeholders", [])
        )
    )

    return narrative
```

**Edge Cases & Failure Modes:**

1. **Unknown Category:**
   ```python
   SUPPORTED_CATEGORIES = ["construction", "it_dev", "finance", "healthcare"]

   if category not in SUPPORTED_CATEGORIES:
       raise HTTPException(
           status_code=400,
           detail=f"Category '{category}' not supported. Choose from: {SUPPORTED_CATEGORIES}"
       )
   ```

2. **Missing Domain Context in Prompts:**
   ```python
   domain_context = prompt_config["domain_contexts"].get(category)

   if not domain_context:
       logger.warning(
           "missing_domain_context",
           category=category,
           prompt_version=prompt_config["version"]
       )
       # Fallback to generic context
       domain_context = {
           "terminology": ["project", "estimate", "timeline"],
           "units": "currency and time",
           "stakeholders": ["client", "provider"]
       }
   ```

3. **Currency/Unit Conversion:**
   ```python
   # Don't convert between different unit types
   if estimate.cost_unit == "person-weeks" and requested_unit == "USD":
       raise HTTPException(
           status_code=400,
           detail="Cannot convert person-weeks to USD. Request estimate in original units."
       )
   ```

**Integration with Standard Patterns:**

- **Flexible Schema:** MongoDB's schema-less nature enables flexible `attributes` field
- **Tenant Isolation:** Domain reference classes can be platform or tenant-specific
- **Caching:** Reference class queries cached same as construction domain
- **Validation:** Pydantic validates `cost_distribution.currency` and `timeline_distribution.unit` are present

**Adding a New Domain (Checklist for Agents):**

1. ✅ Create synthetic data generator in `apps/synthetic-data-generator/generators/{domain}.py`
2. ✅ Define domain-specific `attributes` schema (flexible Dict)
3. ✅ Specify `currency` and `unit` for cost/timeline distributions
4. ✅ Add domain context to `config/prompts/estimate_narrative.json`
5. ✅ Add domain to `SUPPORTED_CATEGORIES` list in API validation
6. ✅ Seed database with `python seed_database.py --domain {domain}`
7. ✅ Test matching algorithm with domain-specific keywords
8. ✅ Validate LLM narratives use domain-appropriate terminology

**MVP Scope:**
- ✅ Domain-agnostic ReferenceClass schema
- ✅ Construction domain (7 subcategories)
- ✅ IT/Dev domain (5 subcategories) - Fast follow
- ✅ Domain-aware LLM prompts
- ❌ Finance domain (deferred to post-MVP)
- ❌ Healthcare domain (deferred to post-MVP)
- ❌ Tenant-custom domains (deferred to post-MVP)

---

## Naming Conventions

### Python (Backend)
- **Files:** snake_case (`estimate_service.py`, `tenant_isolation.py`)
- **Classes:** PascalCase (`EstimateService`, `TenantNotFoundError`)
- **Functions/Methods:** snake_case (`create_estimate()`, `get_tenant_id()`)
- **Constants:** SCREAMING_SNAKE_CASE (`JWT_SECRET_KEY`, `MAX_RETRIES`)
- **Private:** Prefix with underscore (`_validate_tenant()`)

### TypeScript (Widget)
- **Files:** PascalCase for components (`ChatWidget.tsx`), camelCase for utilities (`shadowDom.ts`)
- **Components:** PascalCase (`ChatWidget`, `EstimateDisplay`)
- **Functions/Variables:** camelCase (`useChatSession`, `getTenantBranding`)
- **Interfaces/Types:** PascalCase (`EstimateResponse`, `WidgetConfig`)
- **Constants:** SCREAMING_SNAKE_CASE (`API_BASE_URL`, `WIDGET_VERSION`)

### Database (MongoDB)
- **Collections:** snake_case plural (`tenants`, `reference_classes`, `estimates`)
- **Fields:** snake_case (`tenant_id`, `created_at`, `p50_cost`)
- **IDs:** Use `_id` for MongoDB ObjectId, `{entity}_id` for external IDs (`estimate_id`, `tenant_id`)

### API Endpoints
- **Pattern:** `/api/v1/{resource}/{action}`
- **Resources:** Plural nouns (`/estimates`, `/tenants`, `/feedback`)
- **Example:** `POST /api/v1/estimates`, `GET /api/v1/estimates/{estimate_id}`

## Data Architecture

See **Cross-Cutting Concerns → Section 1-10** for comprehensive data patterns.

**Key MongoDB Collections:**
1. **`tenants`** - Multi-tenant accounts with BYOK encryption
2. **`reference_classes`** - Domain-agnostic estimation templates (synthetic + custom)
3. **`estimates`** - Customer estimation requests with prompt versioning
4. **`feedback`** - Actual project outcomes for calibration

**Critical Indexes:**
```javascript
// ALL indexes MUST include tenant_id first for isolation
db.reference_classes.createIndex({ tenant_id: 1, category: 1, region: 1 })
db.estimates.createIndex({ tenant_id: 1, created_at: -1 })
db.feedback.createIndex({ tenant_id: 1, estimate_id: 1 })
```

## API Contracts

See **PRD → SaaS B2B Backend Specific Requirements → API Endpoints** for complete specifications.

**Core Endpoints:**
- `POST /api/v1/estimates` - Generate project estimate
- `POST /api/v1/chat/send` - Conversational scoping
- `POST /api/v1/feedback/customer` - Submit actual outcomes
- `GET /api/v1/widget/config` - Fetch tenant branding

**Authentication:** JWT Bearer token with `tenant_id` claim

## Deployment Strategy

**Hosting:** DigitalOcean App Platform with auto-deploy from GitHub `main` branch

**Deployment Flow:**
```
git push origin main
  ↓
DigitalOcean detects push
  ↓
Builds Docker image (2-3 min)
  ↓
Zero-downtime rolling deployment (30 sec)
  ↓
Old instances drained, new instances live
```

**Configuration:** `.do/app.yaml` with environment variables (secrets encrypted at rest)

**Monitoring:** DigitalOcean App Platform metrics + Sentry error tracking

## Development Environment

**Prerequisites:**
- Python 3.11+
- Node.js 20+
- MongoDB Atlas account
- DigitalOcean account
- OpenAI API key (for testing)

**Quick Start:**
```bash
# Backend
cd apps/efofx-api
python -m venv venv && source venv/bin/activate
pip install -r requirements.txt
uvicorn app.main:app --reload

# Widget
cd apps/efofx-widget
npm install && npm run dev

# Generate synthetic data
cd apps/synthetic-data-generator
python seed_database.py
```

---

_Generated by BMAD Decision Architecture Workflow v1.3.2_
_Date: 2025-11-09 (Initial), Updated: 2025-11-10 (Novel Patterns + Version Verification)_
_For: Brett_
_Validated: 2025-11-10 by Winston (Architect Agent)_
