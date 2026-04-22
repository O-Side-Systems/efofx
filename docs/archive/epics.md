# efOfX Estimation Service - Epic Breakdown

**Author:** Brett
**Date:** 2025-11-09
**Project Level:** MVP
**Target Scale:** 15 tenants, 100k estimates/month

---

## Overview

This document provides the complete epic and story breakdown for efOfX Estimation Service, decomposing the requirements from the [PRD](./PRD.md) into implementable stories.

This breakdown organizes efOfX Estimation Service development into **7 sequential epics** containing **44-51 stories** total (including 7 manual QA gate stories).

### Epic Overview

| Epic | Title | Value Proposition | Story Count |
|------|-------|-------------------|-------------|
| **1** | Foundation & Infrastructure Setup | Establish project structure, deployment pipeline, and core dependencies | 5 (incl. QA gate) |
| **2** | Reference Class Engine & Synthetic Data | Core estimation capability - match projects to reference classes and calculate P50/P80 estimates | 6-7 (incl. QA gate) |
| **3** | Multi-Tenant Infrastructure & Security | Enable SaaS business model with hard tenant isolation and BYOK | 7-8 (incl. QA gate) |
| **4** | LLM Integration & Conversational Scoping | Transform raw estimates into stakeholder-friendly narratives and enable chat-based project scoping | 6-7 (incl. QA gate) |
| **5** | White Label Chat Widget | Distribution channel - contractors embed widget on their sites to capture leads | 8-9 (incl. QA gate) |
| **6** | Feedback & Calibration System | Self-improving system - actual outcomes refine future estimates | 7-8 (incl. QA gate) |
| **7** | Code Consolidation & Refactoring | DRY/YAGNI compliance, shared libraries, technical debt cleanup | 5 (incl. QA gate) |

**Note:** Each epic includes a final **Manual QA Gate** story that generates a comprehensive test guide for someone with no prior codebase knowledge, suitable for handoff to a QA tester.

### Sequencing Rationale

1. **Epic 1** → Foundation enables all subsequent work (greenfield widget + brownfield backend enhancements)
2. **Epic 2** → Core business logic (RCF estimation engine with synthetic data)
3. **Epic 3** → Security/multi-tenancy foundation before exposing APIs publicly
4. **Epic 4** → AI enhancements bring the "magic" to estimates
5. **Epic 5** → Customer-facing widget (depends on APIs from Epics 2-4)
6. **Epic 6** → Feedback loop completes the self-improving system
7. **Epic 7** → Refactoring with complete context to establish clean patterns and shared libraries

### Project Context

- **Type:** Brownfield MVP (building on existing FastAPI + MongoDB + MCP foundation)
- **Vision:** "Communication coach that happens to estimate projects" - trust through transparency
- **Scale:** 15 tenants, 100k estimates/month, 99.5% uptime
- **Domains:** Construction (MVP), IT/Dev (fast follow)

---

## Epic 1: Foundation & Infrastructure Setup

**Goal:** Establish project structure, deployment pipeline, and core dependencies to enable all subsequent development. This includes initializing the greenfield white label widget (Vite + React + TypeScript) and enhancing the brownfield FastAPI backend with new folder structure and monitoring.

**Value Proposition:** Creates the foundation that enables fast, reliable development of all features. No business value directly, but critical prerequisite for everything else.

---

### Story 1.1: Initialize White Label Widget Project

**As a** developer,
**I want** a modern Vite + React + TypeScript widget project scaffolded with all architectural decisions implemented,
**So that** I can start building the embeddable chat widget on a solid foundation.

**Acceptance Criteria:**

**Given** the architecture specifies Vite + React 19 + TypeScript + Tailwind + Shadow DOM
**When** I run the widget initialization commands
**Then** the project structure is created at `apps/efofx-widget/` with:
- Vite configuration for single `embed.js` bundle output
- React 19 and TypeScript 5.x installed
- Tailwind CSS configured with PostCSS
- `vite-plugin-css-injected-by-js` for inline CSS
- Shadow DOM wrapper component stub
- Package.json with build scripts
- Basic folder structure: `src/components/`, `src/api/`, `src/hooks/`, `src/types/`

**And** running `npm run build` produces a single `dist/embed.js` file

**And** the widget can be embedded in a test HTML page with `<script src="dist/embed.js"></script>`

**Prerequisites:** None (first story)

**Technical Notes:**
- Follow architecture doc: `docs/architecture.md` → Project Initialization → White Label Chat Widget
- Use exact commands from architecture (npm create vite, tailwind init)
- Verify output bundle size is reasonable (<600KB before optimization)
- Document widget initialization pattern in README.md

---

### Story 1.2: Setup Backend Project Structure & Dependencies

**As a** developer,
**I want** the FastAPI backend organized with the architecture-specified folder structure and core dependencies installed,
**So that** I can implement features following consistent patterns.

**Acceptance Criteria:**

**Given** the architecture defines the backend structure at `apps/efofx-api/`
**When** I set up the project structure
**Then** all folders are created following architecture spec:
- `app/api/v1/endpoints/` for API routes
- `app/core/` for config, security, logging
- `app/models/` for Pydantic models
- `app/services/` for business logic
- `app/db/` for MongoDB connection
- `app/middleware/` for tenant isolation, CORS, rate limiting
- `app/utils/` for shared utilities
- `config/prompts/` for git-based prompt JSON files

**And** core dependencies are installed in `requirements.txt`:
- fastapi>=0.100.0
- motor>=3.0.0 (async MongoDB)
- pydantic>=2.0.0
- pydantic-settings (for config)
- python-jose[cryptography] (JWT)
- cryptography>=41.0.0 (Fernet for BYOK)
- structlog>=24.0.0
- slowapi>=0.1.0 (rate limiting)
- sentry-sdk[fastapi]>=2.0.0

**And** `app/main.py` exists with FastAPI app initialization and health endpoint
**And** `app/core/config.py` exists with Pydantic Settings for env vars
**And** running `uvicorn app.main:app --reload` starts the server successfully

**Prerequisites:** Story 1.1 (widget initialized)

**Technical Notes:**
- Use architecture doc folder structure exactly
- Create `.env.example` with all required env vars documented
- Add basic CORS middleware for widget embedding
- Health endpoint: `GET /health` returns `{"status": "healthy"}`

---

### Story 1.3: Configure DigitalOcean Deployment & Auto-Deploy

**As a** developer,
**I want** DigitalOcean App Platform configured with auto-deploy from GitHub,
**So that** pushing to `main` automatically deploys the backend with zero downtime.

**Acceptance Criteria:**

**Given** the architecture specifies DO App Platform with auto-deploy
**When** I create `.do/app.yaml` configuration
**Then** the file includes:
- Service configuration for FastAPI backend (`apps/efofx-api/`)
- Gunicorn run command: `gunicorn -w 2 -k uvicorn.workers.UvicornWorker --bind 0.0.0.0:8080 app.main:app`
- Environment variables placeholders (MONGODB_URI, JWT_SECRET_KEY, etc.)
- Health check endpoint: `/health`
- CPU/memory alerts configured (80% CPU, 90% memory)
- GitHub auto-deploy enabled on `main` branch

**And** when I connect the GitHub repo to DO App Platform
**Then** the app deploys successfully

**And** when I push a commit to `main`
**Then** DigitalOcean triggers automatic deployment within 2-4 minutes

**And** deployment uses zero-downtime rolling updates

**Prerequisites:** Story 1.2 (backend structure ready)

**Technical Notes:**
- Follow architecture doc: Deployment Strategy section
- Set instance size to `basic-xs` (0.5 vCPU, 512 MB RAM) for MVP
- Document environment variables needed in README
- Test deployment with a simple code change to verify auto-deploy works

---

### Story 1.4: Setup MongoDB Atlas & Sentry Monitoring

**As a** developer,
**I want** MongoDB Atlas connected and Sentry error tracking configured,
**So that** I have database persistence and production error visibility.

**Acceptance Criteria:**

**Given** the architecture specifies MongoDB Atlas and Sentry
**When** I configure MongoDB connection
**Then** `app/db/mongodb.py` contains:
- Motor async client initialization
- Connection pool configuration (10-50 connections)
- Database name from env var
- Connection health check function

**And** when the FastAPI app starts
**Then** MongoDB connection is established successfully
**And** health endpoint (`/health`) includes database status check

**And** when I configure Sentry SDK
**Then** `app/main.py` initializes sentry-sdk with:
- DSN from environment variable
- FastAPI integration enabled
- Environment tag (production/staging/development)
- Release version tracking

**And** when I trigger a test error endpoint
**Then** the error appears in Sentry dashboard within 30 seconds

**Prerequisites:** Story 1.3 (deployment configured)

**Technical Notes:**
- MongoDB Atlas free tier (M0) is sufficient for MVP
- Sentry free tier: 5,000 errors/month
- Create indexes in separate migration script (not in connection code)
- Log MongoDB connection status on startup
- Test Sentry with intentional error: `GET /test-error` endpoint

---

### Story 1.5: Manual QA Gate - Foundation Verification

**As a** QA tester with no prior codebase knowledge,
**I want** a comprehensive test guide to verify the foundation epic is complete,
**So that** I can confidently approve Epic 1 before development continues.

**Acceptance Criteria:**

**Given** Epic 1 implementation is complete
**When** I generate the manual QA test guide
**Then** the file `docs/test-guides/epic-1-foundation-qa-guide.md` is created with:

**Section 1: Prerequisites**
- List of required accounts (DigitalOcean, MongoDB Atlas, Sentry, GitHub)
- Required tools (Node.js 20+, Python 3.11+, npm, git)
- Test data requirements (none for Epic 1)

**Section 2: Environment Setup**
- Step-by-step instructions to clone repo
- How to install widget dependencies (`cd apps/efofx-widget && npm install`)
- How to install backend dependencies (`cd apps/efofx-api && pip install -r requirements.txt`)
- How to configure `.env` file with test values

**Section 3: Test Cases**
- **TC1.1:** Widget builds successfully (`npm run build` produces `dist/embed.js`)
- **TC1.2:** Backend starts successfully (`uvicorn app.main:app --reload`)
- **TC1.3:** Health endpoint returns 200 OK (`curl http://localhost:8000/health`)
- **TC1.4:** MongoDB connection status is "connected" in health response
- **TC1.5:** Sentry captures test error (trigger `/test-error`, verify in Sentry dashboard)
- **TC1.6:** DigitalOcean deployment succeeds (push to `main`, verify app is running)
- **TC1.7:** Auto-deploy works (make trivial change, push, verify re-deployment)

**Section 4: Expected Results**
- All test cases pass without errors
- Widget bundle size < 600KB
- Backend responds within 100ms to health check
- MongoDB connection pool established (visible in logs)
- Sentry error appears with full stack trace

**Section 5: Known Limitations**
- No API endpoints implemented yet (only `/health`)
- No database collections created yet
- No authentication/authorization yet
- Widget shows placeholder content only

**And** the test guide includes screenshots for Sentry error verification
**And** the test guide includes command examples with expected output

**Prerequisites:** Stories 1.1-1.4 (all foundation stories complete)

**Technical Notes:**
- Test guide should be runnable by someone unfamiliar with the tech stack
- Include troubleshooting section for common issues
- Provide sample `.env` values (non-sensitive)
- Epic 1 is approved only when Brett confirms all test cases pass

---

## Epic 2: Reference Class Engine & Synthetic Data

**Goal:** Implement the core Reference Class Forecasting (RCF) engine that matches user projects to reference classes and calculates P50/P80 estimates, plus generate synthetic reference data for construction domain using NumPy/SciPy statistical distributions.

**Value Proposition:** This is the CORE business logic - the estimation engine that delivers project cost and timeline forecasts. Without this, there's no product.

---

### Story 2.1: Create Domain-Agnostic Reference Class MongoDB Schema

**As a** developer,
**I want** MongoDB collections and Pydantic models for reference classes that work across any domain,
**So that** the same schema supports construction, IT/dev, and future domains.

**Acceptance Criteria:**

**Given** the architecture specifies domain-agnostic flexible schema
**When** I create the `reference_classes` collection schema
**Then** the Pydantic model in `app/models/reference_class.py` includes:
- `tenant_id`: ObjectId | None (null = platform-provided)
- `category`: str (e.g., "construction", "it_dev")
- `subcategory`: str (e.g., "pool", "api_development")
- `name`: str
- `description`: str
- `keywords`: List[str]
- `regions`: List[str]
- `attributes`: Dict[str, Any] (flexible, domain-specific)
- `cost_distribution`: {p50, p80, p95, currency}
- `timeline_distribution`: {p50_days, p80_days, p95_days}
- `cost_breakdown_template`: Dict[str, float] (percentages)
- `is_synthetic`: bool
- `validation_source`: str
- `created_at`: datetime

**And** MongoDB indexes are created via migration script:
```python
db.reference_classes.createIndex({"tenant_id": 1, "category": 1, "region": 1})
db.reference_classes.createIndex({"tenant_id": 1, "keywords": 1})
```

**And** the schema validates that cost_breakdown_template percentages sum to 1.0

**Prerequisites:** Story 1.4 (MongoDB connected)

**Technical Notes:**
- Follow architecture doc: Data Architecture → MongoDB Collections
- Use Pydantic validators for percentage sum validation
- Store attributes as flexible dict to support any domain
- Example construction attributes: {size_range, depth, includes_spa}
- Example IT attributes: {tech_stack, team_size, complexity}

---

### Story 2.2: Implement RCF Matching Algorithm

**As a** system,
**I want** to match user project descriptions to best reference class using keyword matching and confidence scoring,
**So that** estimates are based on the most relevant historical data.

**Acceptance Criteria:**

**Given** a user provides project description, category, and region
**When** the RCF matching algorithm runs
**Then** `app/services/rcf_engine.py` contains:
- `find_matching_reference_class(description, category, region, tenant_id)` function
- Keyword extraction from description (lowercase, tokenize)
- Scoring logic: keyword overlap + category exact match + region match
- Confidence score calculation (0.0 to 1.0)
- Returns top match with confidence >= 0.7, else None

**And** when confidence < 0.7
**Then** the system returns error suggesting more details needed

**And** when multiple matches have same score
**Then** prefer tenant-specific over platform-provided

**And** the matching algorithm completes in < 50ms (p95)

**Prerequisites:** Story 2.1 (schema created)

**Technical Notes:**
- Use simple keyword overlap for MVP (TF-IDF or ML models post-MVP)
- Scoring formula: (keyword_matches / total_keywords) * 0.6 + category_match * 0.3 + region_match * 0.1
- Cache matching results for identical queries (5 min TTL)
- Log all match attempts with confidence scores for analysis

---

### Story 2.3: Generate Synthetic Construction Reference Classes

**As a** system administrator,
**I want** realistic synthetic reference classes generated for 7 construction project types across 4 regions,
**So that** the MVP has estimation data before real customer feedback exists.

**Acceptance Criteria:**

**Given** FR-2.1 specifies 7 construction types and 4 regions
**When** I run the synthetic data generator script
**Then** `apps/synthetic-data-generator/generators/pool.py` creates:
- Pool reference classes using lognormal cost distributions
- Mean costs based on 2024 HomeAdvisor data
- Regional variations: SoCal Coastal (highest), SoCal Inland (-15%), NorCal (-10%), Central Coast (-5%)
- Timeline distributions using normal distribution
- Cost breakdown templates (materials 40%, labor 30%, equipment 10%, permits 5%, finishing 15%)

**And** generators exist for all 7 types:
- `pool.py`, `adu.py`, `kitchen.py`, `bathroom.py`, `landscaping.py`, `roofing.py`, `flooring.py`

**And** each generator uses reproducible seed: `np.random.seed(42)`

**And** validation script confirms synthetic costs within ±25% of HomeAdvisor 2024 averages

**And** running `python seed_database.py` populates MongoDB with ~100 reference classes (7 types × 4 regions × ~4 size variations)

**Prerequisites:** Story 2.1 (schema ready)

**Technical Notes:**
- Follow architecture doc: Synthetic Data → NumPy/SciPy Distributions
- Use scipy.stats.lognorm for costs (right-skewed, no negatives)
- Use scipy.stats.norm for timelines
- Mark all as `is_synthetic: true`, `tenant_id: null` (platform-provided)
- Document validation sources in each reference class

---

### Story 2.4: Implement Baseline Estimate Calculation

**As a** system,
**I want** to calculate P50/P80 cost and timeline estimates from matched reference class,
**So that** users get probabilistic forecasts instead of single-point estimates.

**Acceptance Criteria:**

**Given** a matched reference class is found
**When** the baseline estimate calculation runs
**Then** `app/services/rcf_engine.py::calculate_baseline_estimate()` returns:
- P50 cost from reference class cost_distribution
- P80 cost from reference class cost_distribution
- P50 timeline (days) from reference class timeline_distribution
- P80 timeline (days) from reference class timeline_distribution
- Cost breakdown using template percentages applied to P50 cost

**And** cost breakdown is returned as dict:
```python
{
  "materials": 30000,  # 40% of 75000
  "labor": 22500,      # 30% of 75000
  "equipment": 7500,   # 10%
  "permits": 3750,     # 5%
  "finishing": 11250   # 15%
}
```

**And** response includes variance range: `{"p50": 75000, "p80": 92000, "variance": 17000}`

**And** calculation completes in < 10ms

**Prerequisites:** Story 2.3 (reference classes exist)

**Technical Notes:**
- No adjustments applied yet (Story 2.5 handles that)
- Breakdown percentages must sum to P50 cost exactly (handle rounding)
- Return Pydantic model: `EstimateResponse` with all fields
- Log which reference class was used for each estimate (traceability)

---

### Story 2.5: Apply Regional & Complexity Adjustments

**As a** system,
**I want** to apply regional multipliers and complexity/risk factors to baseline estimates,
**So that** estimates account for location and project-specific factors.

**Acceptance Criteria:**

**Given** a baseline estimate is calculated
**When** regional and complexity adjustments are applied
**Then** `app/services/rcf_engine.py::apply_adjustments()` modifies:
- Regional multiplier (already baked into reference class, verify it's applied)
- Complexity factor: 0.8x (simple) to 1.5x (complex) based on user input
- Risk factor: 1.0x (low) to 1.3x (high) based on project attributes

**And** final cost = baseline × complexity × risk

**And** final timeline = baseline_timeline × complexity (risk doesn't affect timeline)

**And** calculation breakdown is returned showing:
```python
{
  "baseline_p50": 75000,
  "complexity_factor": 1.2,
  "risk_factor": 1.1,
  "adjusted_p50": 99000  # 75000 * 1.2 * 1.1
}
```

**And** when complexity or risk is not provided
**Then** defaults to 1.0 (no adjustment)

**Prerequisites:** Story 2.4 (baseline calculation works)

**Technical Notes:**
- Complexity factor from user input: "simple" (0.8), "standard" (1.0), "complex" (1.5)
- Risk factor from heuristics: foundation issues, tight timeline, custom requirements
- Store adjustment factors in estimate document for transparency
- Apply multiplicatively: final = baseline × regional × complexity × risk

---

### Story 2.6: Manual QA Gate - Estimation Engine Verification

**As a** QA tester,
**I want** a test guide to verify the RCF engine produces accurate, explainable estimates,
**So that** I can approve Epic 2 before widget integration begins.

**Acceptance Criteria:**

**Given** Epic 2 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-2-rcf-engine-qa-guide.md` includes:

**Section 1: Prerequisites**
- MongoDB populated with synthetic data (100 reference classes)
- Test API client (Postman, curl, or Python requests)
- Sample project descriptions for testing

**Section 2: Test Cases**
- **TC2.1:** Match "pool installation in SoCal coastal" → confidence >= 0.8
- **TC2.2:** Match ambiguous "backyard project" → confidence < 0.7, error returned
- **TC2.3:** Baseline estimate for matched pool → P50 ~$75k, P80 ~$92k (±10% tolerance)
- **TC2.4:** Cost breakdown sums to P50 exactly
- **TC2.5:** Apply complexity=1.2 → adjusted cost = baseline * 1.2
- **TC2.6:** Apply risk=1.1 → adjusted cost = baseline * complexity * risk
- **TC2.7:** Verify all 7 construction types have reference classes
- **TC2.8:** Verify all 4 regions represented for each type
- **TC2.9:** Response time < 100ms for estimate calculation
- **TC2.10:** Synthetic data validation: costs within ±25% of HomeAdvisor 2024

**Section 3: Example API Requests**
```bash
curl -X POST http://localhost:8000/api/v1/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "project_type": "Pool Installation",
    "region": "SoCal - Coastal",
    "description": "15x30 in-ground pool with spa",
    "complexity": "standard"
  }'
```

**Section 4: Expected Results**
- Match confidence scores make sense (higher for specific descriptions)
- P50/P80 values are realistic compared to industry averages
- Cost breakdowns are reasonable (materials typically 40-50% for construction)
- Adjustments multiply correctly
- All reference classes marked `is_synthetic: true`

**Section 5: Known Limitations**
- No real project data yet (all synthetic)
- Simple keyword matching (no ML/NLP)
- No LLM narrative generation yet (Epic 4)
- No user authentication yet (Epic 3)

**Prerequisites:** Stories 2.1-2.5

**Technical Notes:**
- Include HomeAdvisor 2024 pool cost range for validation: $50k-$120k (SoCal)
- Verify lognormal distribution shape (right-skewed, no negative costs)
- Test edge cases: empty description, unknown region, invalid category

---

## Epic 3: Multi-Tenant Infrastructure & Security

**Goal:** Implement multi-tenant SaaS foundation with hard tenant isolation, JWT authentication, BYOK encryption for OpenAI keys, and per-tenant rate limiting. This enables the business model and ensures zero cross-tenant data leakage.

**Value Proposition:** Transforms the application into a true multi-tenant SaaS platform where multiple contractors can use the service securely and independently.

---

### Story 3.1: Create Tenant Registration & Management

**As a** contractor (tenant),
**I want** to register for an efOfX account with email verification,
**So that** I can access the estimation service and manage my settings.

**Acceptance Criteria:**

**Given** the PRD specifies tenant registration with tiers
**When** I implement tenant management
**Then** the Pydantic model `app/models/tenant.py` includes:
- `_id`: ObjectId
- `name`: str (company name)
- `email`: str (unique, validated)
- `password_hash`: str (bcrypt)
- `tier`: Enum[trial, pro, enterprise]
- `api_key_hash`: str
- `openai_api_key_encrypted`: str | None
- `branding`: dict (logo_url, primary_color, widget_button_text)
- `rate_limit_tier`: str
- `is_active`: bool
- `created_at`, `updated_at`: datetime

**And** `POST /api/v1/tenants` endpoint creates tenant:
- Validates email format and uniqueness
- Hashes password with bcrypt (cost factor 12)
- Generates unique API key (hashed with bcrypt for storage)
- Sends verification email via SendGrid
- Returns tenant_id and API key (plaintext, only shown once)

**And** `PATCH /api/v1/tenants/{id}` updates tenant settings (name, branding, tier)

**And** `GET /api/v1/tenants/{id}` returns tenant info (excluding password_hash and openai_api_key_encrypted)

**Prerequisites:** Story 1.4 (MongoDB connected)

**Technical Notes:**
- Never return plaintext API key after initial creation
- Store API key hash only (bcrypt), not reversible
- Email verification token expires after 24 hours
- Trial tier is default, no credit card required

---

### Story 3.2: Implement JWT Authentication & Token Management

**As a** developer,
**I want** JWT-based authentication for all API endpoints,
**So that** requests are authenticated and tenant_id is available in all operations.

**Acceptance Criteria:**

**Given** the architecture specifies JWT with tenant_id claim
**When** I implement authentication
**Then** `app/core/security.py` contains:
- `create_access_token(user_id, tenant_id, role)` function
- `verify_token(token)` function with expiration check
- JWT secret key from environment variable
- Token expiration: 24 hours (access), 30 days (refresh)

**And** JWT token structure includes:
```python
{
  "sub": "user_id_123",
  "tenant_id": "tenant_abc",
  "role": "admin",
  "exp": 1699564800
}
```

**And** `app/api/v1/dependencies.py` contains:
- `get_current_user()` dependency that extracts user from JWT
- `get_tenant_id()` dependency that extracts tenant_id from JWT
- Raises 401 Unauthorized if token invalid/expired

**And** `POST /api/v1/auth/login` endpoint:
- Accepts email + password
- Verifies credentials
- Returns access_token and refresh_token

**And** all protected endpoints use `tenant_id = Depends(get_tenant_id)` pattern

**Prerequisites:** Story 3.1 (tenant model exists)

**Technical Notes:**
- Use python-jose[cryptography] for JWT
- Include rate limit check in login endpoint (5 attempts per 15 min)
- Log all authentication attempts (success and failure)
- Consider refresh token rotation for security

---

### Story 3.3: Implement BYOK Encryption for OpenAI Keys

**As a** contractor (tenant),
**I want** to store my OpenAI API key securely encrypted,
**So that** LLM requests use my key and I control my usage costs.

**Acceptance Criteria:**

**Given** the architecture specifies AES-256 Fernet encryption for BYOK
**When** I implement BYOK storage
**Then** `app/core/security.py` contains:
- `encrypt_openai_key(plaintext_key, tenant_id)` function using Fernet
- `decrypt_openai_key(tenant_id)` function (decrypts at request time only)
- Encryption key loaded from environment variable (ENCRYPTION_KEY)
- Keys never logged or transmitted in plaintext

**And** `POST /api/v1/tenants/{id}/openai-key` endpoint:
- Accepts OpenAI API key in request body
- Validates key format (starts with "sk-")
- Tests key with simple OpenAI API call (validation)
- Encrypts key with Fernet
- Stores encrypted key in tenant document
- Returns success (does not echo key back)

**And** when LLM service needs key
**Then** it calls `decrypt_openai_key(tenant_id)` to get plaintext in memory

**And** when key is invalid/expired
**Then** API returns 402 Payment Required with helpful error message

**Prerequisites:** Story 3.1 (tenant model has openai_api_key_encrypted field)

**Technical Notes:**
- Generate Fernet key: `python -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())"`
- Store ENCRYPTION_KEY in DO environment variables (encrypted at rest)
- Trial tier can use platform key (fallback), Pro/Enterprise require BYOK
- Audit log all key changes (encrypt, update, remove)

---

### Story 3.4: Enforce Tenant Isolation Middleware

**As a** system,
**I want** ALL database queries to include tenant_id filter automatically,
**So that** cross-tenant data leakage is impossible.

**Acceptance Criteria:**

**Given** the architecture mandates 100% tenant_id scoping
**When** I create tenant isolation middleware
**Then** `app/middleware/tenant_isolation.py` contains:
- Middleware that extracts tenant_id from JWT on every request
- Attaches tenant_id to request state: `request.state.tenant_id`
- Raises 401 if tenant_id missing (except for public endpoints like login)

**And** all service methods accept tenant_id as FIRST parameter:
```python
async def get_reference_classes(tenant_id: str, category: str):
    return await db.reference_classes.find({
        "tenant_id": tenant_id,  # REQUIRED
        "category": category
    }).to_list(None)
```

**And** MongoDB queries ALWAYS include tenant_id:
```python
# ✅ CORRECT
{"tenant_id": tenant_id, "status": "active"}

# ❌ WRONG - will fail code review
{"status": "active"}
```

**And** code review checklist includes: "Does this query filter by tenant_id?"

**And** integration tests verify tenant A cannot access tenant B's data

**Prerequisites:** Story 3.2 (JWT includes tenant_id)

**Technical Notes:**
- Follow architecture: Cross-Cutting Concerns → Multi-Tenant Isolation
- Public endpoints (login, health check) bypass tenant_id requirement
- Create helper function: `get_tenant_filter(tenant_id, **kwargs)` that merges filters
- Write pytest fixtures for tenant isolation testing

---

### Story 3.5: Implement Per-Tenant Rate Limiting

**As a** system,
**I want** to enforce rate limits based on tenant tier,
**So that** free tiers don't consume excessive resources and paid tiers get higher quotas.

**Acceptance Criteria:**

**Given** FR-3.4 specifies tier-based rate limits
**When** I implement rate limiting
**Then** `app/middleware/rate_limiting.py` uses slowapi:
- Trial: 10 requests/hour
- Pro: 100 requests/hour
- Enterprise: 1000 requests/hour

**And** rate limit key function extracts tenant_id from JWT:
```python
from slowapi import Limiter
limiter = Limiter(key_func=lambda: get_tenant_id_from_request())
```

**And** endpoints are decorated with dynamic limits:
```python
@app.post("/estimate")
@limiter.limit(lambda: get_tenant_rate_limit())
async def create_estimate(...):
    pass
```

**And** when rate limit exceeded
**Then** API returns 429 Too Many Requests with:
- Current limit and window
- Retry-After header (seconds until reset)
- Upgrade URL for higher tier

**And** rate limit headers included in all responses:
- X-RateLimit-Limit
- X-RateLimit-Remaining
- X-RateLimit-Reset

**Prerequisites:** Story 3.1 (tenant tier field exists)

**Technical Notes:**
- Use Redis for rate limit counters (optional for MVP, in-memory acceptable)
- Rate limit check adds <5ms latency per request
- Global rate limit: 1000 concurrent requests across platform
- Widget sessions: 50 concurrent per tenant (separate limit)

---

### Story 3.6: Create MongoDB Indexes for Tenant Isolation

**As a** system,
**I want** MongoDB indexes optimized for tenant-scoped queries,
**So that** multi-tenant queries are fast even with millions of documents.

**Acceptance Criteria:**

**Given** all queries include tenant_id as first filter
**When** I create database indexes
**Then** migration script `app/db/migrations/001_tenant_indexes.py` creates:

```python
# Tenants collection
db.tenants.createIndex({"email": 1}, {"unique": true})
db.tenants.createIndex({"api_key_hash": 1})

# Reference classes (tenant_id ALWAYS first)
db.reference_classes.createIndex({"tenant_id": 1, "category": 1, "regions": 1})
db.reference_classes.createIndex({"tenant_id": 1, "keywords": 1})

# Estimates
db.estimates.createIndex({"tenant_id": 1, "created_at": -1})
db.estimates.createIndex({"tenant_id": 1, "estimate_id": 1}, {"unique": true})

# Feedback
db.feedback.createIndex({"tenant_id": 1, "estimate_id": 1})
db.feedback.createIndex({"tenant_id": 1, "created_at": -1})
```

**And** index creation is idempotent (safe to run multiple times)

**And** query explain plans show index usage for tenant-scoped queries

**And** query performance: tenant-scoped queries < 50ms (p95)

**Prerequisites:** Stories 2.1, 3.1 (collections exist)

**Technical Notes:**
- Run migrations on app startup (check if indexes exist first)
- Compound indexes with tenant_id first enable efficient tenant isolation
- Monitor index size and query patterns in MongoDB Atlas
- For null tenant_id (platform data), use sparse indexes if needed

---

### Story 3.7: Manual QA Gate - Multi-Tenant Security Verification

**As a** QA tester,
**I want** a comprehensive test guide to verify tenant isolation and security controls,
**So that** I can confirm no cross-tenant data leakage before widget goes live.

**Acceptance Criteria:**

**Given** Epic 3 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-3-multi-tenant-qa-guide.md` includes:

**Section 1: Prerequisites**
- Two test tenant accounts created (Tenant A: trial, Tenant B: pro)
- Each tenant has unique API key
- Each tenant has BYOK OpenAI key configured
- Test estimates created for both tenants

**Section 2: Test Cases - Authentication**
- **TC3.1:** Register new tenant → receives verification email
- **TC3.2:** Login with valid credentials → receives JWT token
- **TC3.3:** Login with invalid credentials → 401 error after 5 attempts → rate limited
- **TC3.4:** Use expired JWT token → 401 Unauthorized
- **TC3.5:** JWT token includes tenant_id claim

**Section 3: Test Cases - Tenant Isolation**
- **TC3.6:** Tenant A creates estimate → estimate has tenant_id = A
- **TC3.7:** Tenant B queries estimates → only sees own estimates (not A's)
- **TC3.8:** Tenant A tries to access Tenant B's estimate by ID → 404 Not Found (not 403, to prevent info leak)
- **TC3.9:** Platform-provided reference classes visible to all tenants (tenant_id = null)
- **TC3.10:** Tenant-specific reference classes only visible to owning tenant

**Section 4: Test Cases - BYOK Security**
- **TC3.11:** Store OpenAI key → encrypted value in database (not plaintext)
- **TC3.12:** Retrieve key for LLM use → decrypts successfully
- **TC3.13:** Invalid OpenAI key → validation fails on store
- **TC3.14:** OpenAI key never appears in logs or error messages
- **TC3.15:** Update OpenAI key → old key overwritten, audit log created

**Section 5: Test Cases - Rate Limiting**
- **TC3.16:** Trial tenant makes 10 requests in 1 hour → 11th request returns 429
- **TC3.17:** Pro tenant makes 100 requests in 1 hour → 101st returns 429
- **TC3.18:** Rate limit headers present in response (X-RateLimit-*)
- **TC3.19:** After rate limit reset → requests succeed again
- **TC3.20:** Rate limit error includes upgrade URL

**Section 6: Test Cases - Database Performance**
- **TC3.21:** Query explain plan for tenant-scoped query → uses index (tenant_id, ...)
- **TC3.22:** Tenant query response time < 50ms with 10,000 documents
- **TC3.23:** All indexes created successfully (check MongoDB Atlas)

**Section 7: Expected Results**
- Zero cross-tenant data access possible
- All passwords bcrypt hashed (cost factor 12)
- All OpenAI keys Fernet encrypted (AES-256)
- JWT tokens expire after 24 hours
- Rate limits enforce tier restrictions
- Database queries use tenant_id indexes

**Section 8: Known Limitations**
- No MFA yet (Enterprise feature, post-MVP)
- No audit log UI yet (logs in MongoDB only)
- No tenant deactivation workflow yet

**Prerequisites:** Stories 3.1-3.6

**Technical Notes:**
- Security audit must confirm zero data leakage
- Use tools like Postman to test with different tenant JWT tokens
- Verify encrypted values in MongoDB look like gibberish (Fernet format)
- Test concurrent requests from multiple tenants

---

---

## Epic 4: LLM Integration & Conversational Scoping

**Goal:** Integrate OpenAI with BYOK to generate stakeholder-friendly estimate narratives and enable conversational project scoping through chat interface. Manage prompts as version-controlled JSON files.

**Value Proposition:** Adds the AI "magic" that transforms raw estimates into transparent, educational explanations and makes project scoping feel like talking to a helpful expert.

---

### Story 4.1: Create OpenAI Client with BYOK Support

**As a** developer,
**I want** an OpenAI client that uses tenant-specific API keys from BYOK storage,
**So that** LLM requests are billed to the tenant and respect their quota.

**Acceptance Criteria:**

**Given** the architecture specifies OpenAI BYOK with retry logic
**When** I implement the LLM service
**Then** `app/services/llm_service.py` contains:
- `LLMService` class with async OpenAI client
- `get_openai_client(tenant_id)` that decrypts tenant's key
- Fallback to platform key for Trial tier tenants
- Retry decorator with exponential backoff (max 3 retries)
- Timeout: 30 seconds per request

**And** when making LLM requests
**Then** the service includes:
- Tenant_id in request metadata (for tracking)
- Model selection from config (gpt-4o-mini for MVP)
- Structured output mode support (JSON)
- Error handling for rate limits (429), quota exceeded (402), invalid key (401)

**And** when tenant's OpenAI key fails
**Then** helpful error returned: "Your OpenAI API key quota exceeded. Please add credits or upgrade."

**And** LLM requests log: tenant_id, model, prompt_version, tokens used, latency

**Prerequisites:** Story 3.3 (BYOK encryption implemented)

**Technical Notes:**
- Use openai>=1.0.0 (latest SDK)
- Cache LLM responses for identical prompts (1 hour TTL)
- Monitor OpenAI API status and circuit break if down
- Consider streaming responses for long narratives (post-MVP)

---

### Story 4.2: Implement Git-Based Prompt Management

**As a** developer,
**I want** prompts stored as version-controlled JSON files,
**So that** prompt changes are tracked, reviewable, and each estimate links to its prompt version.

**Acceptance Criteria:**

**Given** the architecture specifies git-based JSON prompts
**When** I create prompt files
**Then** `config/prompts/` contains:
- `estimate_narrative.json` - P50/P80 explanation prompt
- `conversational_scoping.json` - Chat bot prompt
- `feedback_analysis.json` - Calibration feedback prompt (future)

**And** each prompt file structure:
```json
{
  "version": "1.0.0",
  "created_at": "2025-11-09T10:00:00Z",
  "model": "gpt-4o-mini",
  "system_prompt": "You are an expert project estimator...",
  "user_template": "Generate narrative for: {project_type}...",
  "temperature": 0.7,
  "max_tokens": 500
}
```

**And** `app/utils/prompt_loader.py` contains:
- `PromptLoader` class that reads JSON files
- In-memory cache of loaded prompts
- `render(name, **kwargs)` that formats user_template with variables

**And** when generating estimate
**Then** prompt version stored in estimate document: `{"prompt_version": "1.0.0"}`

**And** when prompt file changes
**Then** git commit tracks the change with reason

**Prerequisites:** Story 4.1 (LLM client ready)

**Technical Notes:**
- Prompts are code - require PR review before merge
- Use semantic versioning for prompts (1.0.0, 1.1.0, 2.0.0)
- Store prompt_version with every estimate for calibration analysis
- Post-MVP: Migrate to LangSmith for A/B testing and analytics

---

### Story 4.3: Generate Estimate Narratives with LLM

**As a** user,
**I want** my estimate explained in plain language with assumptions and risks highlighted,
**So that** I understand what the numbers mean and what might change them.

**Acceptance Criteria:**

**Given** a baseline estimate is calculated
**When** I request narrative generation
**Then** `app/services/llm_service.py::generate_estimate_narrative()`:
- Loads `estimate_narrative.json` prompt
- Renders prompt with: project_type, region, p50_cost, p80_cost, breakdown
- Calls OpenAI API with rendered prompt
- Returns narrative (150-300 words)
- Stores prompt_version in estimate document

**And** narrative includes:
- Explanation of P50 vs P80 ("50% of projects cost under $75k, 80% under $92k")
- Key assumptions ("Assumes standard soil conditions, permits obtained")
- Risk factors ("Pool depth or custom features could add $5-15k")
- Next steps ("Get 3 contractor quotes, verify permits")

**And** when LLM call fails
**Then** graceful degradation: return estimate without narrative, log error to Sentry

**And** narrative generation adds <3 seconds to estimate API response time

**Prerequisites:** Story 4.2 (prompt loader ready)

**Technical Notes:**
- Use temperature=0.7 for some creativity while staying factual
- Max tokens=500 keeps narratives concise
- Consider streaming for instant user feedback (post-MVP)
- A/B test prompt versions for user satisfaction

---

### Story 4.4: Implement Conversational Scoping Chat Engine

**As a** user,
**I want** to describe my project in natural language through chat,
**So that** the system gathers details without forcing me to fill out forms.

**Acceptance Criteria:**

**Given** the widget needs conversational project scoping
**When** I implement chat engine
**Then** `app/services/chat_service.py` contains:
- `ChatSession` class that maintains conversation state
- `send_message(session_id, user_message, tenant_id)` function
- Loads `conversational_scoping.json` prompt
- Uses OpenAI function calling to extract structured data
- Returns bot response + extracted attributes
- Determines when enough info exists for estimate (confidence >= 0.7)

**And** `POST /api/v1/chat/send` endpoint:
- Accepts: session_id, message, tenant_id (from JWT)
- Returns: bot_message, extracted_attributes, ready_for_estimate
- Stores conversation history in MongoDB

**And** conversation flow:
1. Bot: "Hi! What project are you planning?"
2. User: "Pool installation"
3. Bot: "Great! What's the approximate size?" → extracts project_type="Pool"
4. User: "15x30 feet"
5. Bot: "Where are you located?" → extracts size="15x30"
6. User: "San Diego"
7. Bot: "Got it! Let me calculate..." → ready_for_estimate=true

**And** when insufficient info
**Then** bot asks clarifying questions based on missing attributes

**Prerequisites:** Story 4.2 (prompt loader ready)

**Technical Notes:**
- Use OpenAI structured outputs (JSON mode) for attribute extraction
- Session expires after 24 hours of inactivity
- Store chat history: {session_id, tenant_id, messages[], extracted_attributes}
- Widget calls this API for each user message

---

### Story 4.5: Implement LLM Response Caching

**As a** system,
**I want** to cache identical LLM prompts to avoid redundant API calls,
**So that** costs are reduced and response times are faster for common queries.

**Acceptance Criteria:**

**Given** many users request estimates for similar projects
**When** I implement LLM caching
**Then** `app/utils/cache.py` contains:
- `LLMCache` class using in-memory LRU cache
- Cache key = hash(prompt + model + temperature)
- Cache TTL = 1 hour
- Cache hit → return cached response immediately
- Cache miss → call OpenAI, store response

**And** when cache hit occurs
**Then** response time < 50ms (vs 2-3s for OpenAI call)

**And** when cache is full (max 1000 entries)
**Then** LRU eviction removes oldest unused entry

**And** cache metrics logged:
- Hit rate percentage
- Total hits/misses
- Average response time (cached vs uncached)

**And** cache bypass available via request header: `X-Bypass-Cache: true`

**Prerequisites:** Story 4.3 (narrative generation works)

**Technical Notes:**
- Use Python functools.lru_cache or Redis (if available)
- Cache only successful LLM responses (not errors)
- Consider cache warm-up for popular project types
- Monitor hit rate - should be >30% for cost savings

---

### Story 4.6: Manual QA Gate - LLM Integration Verification

**As a** QA tester,
**I want** a test guide to verify LLM integration produces helpful, accurate narratives,
**So that** I can approve Epic 4 before widget development begins.

**Acceptance Criteria:**

**Given** Epic 4 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-4-llm-integration-qa-guide.md` includes:

**Section 1: Prerequisites**
- Test tenant with valid OpenAI API key configured
- Platform OpenAI key for Trial tier testing
- Sample project descriptions (pool, ADU, kitchen remodel)
- Access to OpenAI usage dashboard

**Section 2: Test Cases - BYOK**
- **TC4.1:** Pro tenant estimate uses tenant's OpenAI key (verify in OpenAI dashboard)
- **TC4.2:** Trial tenant estimate uses platform key (fallback)
- **TC4.3:** Invalid OpenAI key → helpful error message (not raw API error)
- **TC4.4:** OpenAI quota exceeded → 402 error with upgrade suggestion
- **TC4.5:** OpenAI rate limit (429) → retry succeeds on 2nd/3rd attempt

**Section 3: Test Cases - Narrative Quality**
- **TC4.6:** Estimate narrative includes P50/P80 explanation
- **TC4.7:** Narrative includes 3+ assumptions
- **TC4.8:** Narrative includes 2+ risk factors
- **TC4.9:** Narrative suggests next steps
- **TC4.10:** Narrative length 150-300 words
- **TC4.11:** Tone is professional, transparent, educational (not salesy)

**Section 4: Test Cases - Conversational Scoping**
- **TC4.12:** Chat bot greets user and asks about project
- **TC4.13:** User says "pool" → bot asks about size
- **TC4.14:** User provides size/location → bot has enough info (ready_for_estimate=true)
- **TC4.15:** Extracted attributes match user input (project_type="Pool", size="15x30", region="San Diego")
- **TC4.16:** Ambiguous input → bot asks clarifying question
- **TC4.17:** Chat history persists across messages (session_id)

**Section 5: Test Cases - Prompt Versioning**
- **TC4.18:** Estimate document includes prompt_version (e.g., "1.0.0")
- **TC4.19:** Change prompt JSON → new estimates use new version
- **TC4.20:** Old estimates retain original prompt_version for calibration

**Section 6: Test Cases - Caching**
- **TC4.21:** Identical prompt → 2nd request returns cached response (<50ms)
- **TC4.22:** Cache hit rate >30% after 100 test requests
- **TC4.23:** Cache respects TTL (response refreshes after 1 hour)

**Section 7: Expected Results**
- BYOK keys used correctly per tenant tier
- Narratives are helpful and understandable by non-technical users
- Chat bot extracts key attributes accurately
- Prompt versioning enables calibration analysis
- Caching reduces OpenAI costs by 30%+

**Section 8: Known Limitations**
- Simple keyword extraction (no advanced NLP yet)
- No streaming responses (entire narrative at once)
- No multi-language support yet
- No prompt A/B testing yet

**Prerequisites:** Stories 4.1-4.5

**Technical Notes:**
- Review 10+ generated narratives for quality and consistency
- Test with edge cases: very large projects, unusual regions, custom features
- Verify OpenAI token usage matches expectations (~500 tokens per narrative)

---

---

## Epic 5: White Label Chat Widget

**Goal:** Create embeddable JavaScript widget that contractors can add to their websites with <5 lines of code. Widget provides conversational project scoping UI, captures leads, and generates estimates - all with full branding customization and no efOfX branding visible.

**Value Proposition:** Primary distribution channel for efOfX. Enables contractors to offer AI-powered estimation directly on their sites, capturing leads while providing instant value to visitors.

---

### Story 5.1: Implement Shadow DOM Widget Container

**As a** developer,
**I want** the widget to render in Shadow DOM with isolated styles,
**So that** it doesn't conflict with the host website's CSS and works reliably across all sites.

**Acceptance Criteria:**

**Given** the architecture specifies Shadow DOM for style isolation
**When** I implement the widget container
**Then** `apps/efofx-widget/src/components/WidgetContainer.tsx` contains:
- Shadow DOM root creation and attachment
- Style isolation (Tailwind styles don't leak to host page)
- Widget mount/unmount lifecycle
- Z-index management for overlay positioning
- Responsive container that adapts to screen size

**And** the widget initialization in `apps/efofx-widget/src/main.tsx`:
```typescript
class EfofxWidget {
  init(config: WidgetConfig) {
    const container = document.createElement('div');
    const shadow = container.attachShadow({ mode: 'open' });
    // Mount React app in Shadow DOM
  }
}
window.efofxWidget = new EfofxWidget();
```

**And** when embedded on any website
**Then** host page styles don't affect widget appearance
**And** widget styles don't affect host page

**And** widget renders correctly on mobile (320px+) and desktop (1920px+)

**Prerequisites:** Story 1.1 (widget project initialized)

**Technical Notes:**
- Follow architecture: Widget → Shadow DOM for Style Isolation
- Use `adoptedStyleSheets` for injecting Tailwind CSS into Shadow DOM
- Test on sites with Bootstrap, Material-UI, Tailwind to verify isolation
- Handle iframe edge cases if host uses CSP

---

### Story 5.2: Create Widget Branding Configuration System

**As a** contractor (tenant),
**I want** to customize the widget's appearance to match my brand,
**So that** customers see my branding, not efOfX's.

**Acceptance Criteria:**

**Given** FR-6.2 specifies branding customization
**When** I implement branding configuration
**Then** `GET /api/v1/widget/config?api_key={key}` endpoint returns:
```json
{
  "branding": {
    "logo_url": "https://acmecontractors.com/logo.png",
    "primary_color": "#FF5733",
    "button_text": "Get Free Estimate",
    "welcome_message": "Hi! Let's estimate your project."
  },
  "session_token": "uuid-v4"
}
```

**And** widget initialization fetches branding:
```typescript
efofxWidget.init({
  apiKey: 'efofx_abc123',
  onReady: (branding) => {
    // Apply branding to widget UI
  }
});
```

**And** branding is applied dynamically:
- Logo displayed in widget header
- Primary color used for buttons, chat bubbles, accents
- Button text customizable
- Welcome message personalized
- No efOfX branding visible anywhere

**And** tenant can preview branding changes in dashboard before publishing

**And** branding changes propagate to all widget instances within 5 minutes (CDN cache)

**Prerequisites:** Story 3.1 (tenant branding stored), Story 5.1 (widget container ready)

**Technical Notes:**
- Cache branding config in widget localStorage (24 hour TTL)
- Validate color format (hex, rgb, hsl)
- Logo dimensions: max 200x60px, auto-scaled
- Fallback to default branding if fetch fails
- Preview mode: `?preview_branding=tenant_id` for testing

---

### Story 5.3: Implement Conversational Chat UI

**As a** website visitor,
**I want** to describe my project in natural language through chat,
**So that** I get an estimate without filling out boring forms.

**Acceptance Criteria:**

**Given** FR-6.3 specifies conversational UI
**When** I implement the chat interface
**Then** `apps/efofx-widget/src/components/Chat.tsx` contains:
- Message list with auto-scroll to bottom
- User message bubbles (right-aligned, primary color)
- Bot message bubbles (left-aligned, gray)
- Typing indicator animation while bot is responding
- Quick-select suggestion buttons
- Input field with send button
- Mobile-optimized keyboard handling

**And** chat flow:
1. Widget opens → Welcome message displayed
2. User types message → Sent to `/api/v1/chat/send`
3. Bot responds → Message + suggestions displayed
4. Loop continues until `ready_for_estimate: true`
5. Estimate generated and displayed in chat

**And** when bot provides suggestions
**Then** quick-select buttons render below message:
```tsx
<SuggestionButton onClick={() => sendMessage("15x30 feet")}>
  15x30 feet
</SuggestionButton>
```

**And** message history persists in session (survives page refresh for 24 hours)

**And** response time: bot messages appear within 2 seconds

**Prerequisites:** Story 4.4 (chat API ready), Story 5.2 (branding applied)

**Technical Notes:**
- Use React 19 `useTransition` for optimistic UI updates
- Store chat history in sessionStorage (session_token key)
- Implement message retry on network failure
- Accessibility: ARIA labels, keyboard navigation
- Mobile: Input field fixed at bottom, prevents iOS keyboard jump

---

### Story 5.4: Implement Lead Capture Form

**As a** contractor,
**I want** to capture visitor email/phone before showing estimate,
**So that** I can follow up on leads even if they don't book immediately.

**Acceptance Criteria:**

**Given** FR-6.4 specifies configurable lead capture
**When** I implement lead capture
**Then** `apps/efofx-widget/src/components/LeadCapture.tsx` displays form when:
- Chat reaches `ready_for_estimate: true`
- Lead info not yet captured for this session

**And** form fields:
- Email (required, validated)
- Phone (optional, formatted)
- Name (optional)
- Privacy policy agreement checkbox (required if GDPR applies)

**And** tenant configuration controls:
```json
{
  "lead_capture": {
    "enabled": true,
    "email_required": true,
    "phone_required": false,
    "timing": "before_estimate"  // or "after_estimate"
  }
}
```

**And** when lead submits form
**Then** `POST /api/v1/widget/leads` saves:
- Email, phone, name
- Session_id, tenant_id
- Captured_at timestamp
- Associated estimate_id (after estimate generated)

**And** tenant receives email notification: "New lead: john@example.com requested pool estimate"

**And** lead receives email with estimate summary and contractor contact info

**Prerequisites:** Story 5.3 (chat UI ready)

**Technical Notes:**
- Email validation: RFC 5322 compliant regex
- Phone formatting: support US/international formats
- GDPR compliance: Privacy policy link required for EU visitors (use geo-IP)
- Lead deduplication: Same email + tenant within 7 days = existing lead
- A/B test lead capture timing (before vs after estimate)

---

### Story 5.5: Display Estimate Results in Widget

**As a** website visitor,
**I want** to see the estimate clearly formatted in the chat,
**So that** I understand the cost range and next steps.

**Acceptance Criteria:**

**Given** chat session reaches estimate-ready state
**When** estimate is generated
**Then** `apps/efofx-widget/src/components/EstimateDisplay.tsx` renders:
- Project summary (type, size, region)
- P50 cost: "$75,000 (50% of projects)"
- P80 cost: "$92,000 (80% of projects)"
- Range visualization (horizontal bar showing P50/P80 markers)
- Timeline: "8-11 weeks typical"
- Cost breakdown (collapsible accordion): materials, labor, equipment, etc.
- Assumptions list (collapsible)
- Risks list (collapsible)
- LLM narrative (150-300 words)
- "Request Detailed Quote" CTA button (emails contractor)

**And** estimate layout is mobile-responsive:
- Stacked vertically on mobile (<640px)
- Two-column on desktop (>=640px)

**And** when user clicks "Request Detailed Quote"
**Then** email sent to contractor with:
- Lead info (email, phone)
- Project description
- Estimate summary
- Timestamp

**And** user sees confirmation: "Your request has been sent! {Contractor} will contact you soon."

**Prerequisites:** Story 4.3 (estimate API with narrative), Story 5.4 (lead captured)

**Technical Notes:**
- Use Chart.js or Recharts for P50/P80 visualization
- Collapsible sections default to collapsed on mobile
- Print-friendly CSS (optional: "Download PDF" button)
- Track conversion: lead capture → estimate view → quote request
- Store estimate in sessionStorage for revisiting

---

### Story 5.6: Implement Widget Security Hardening

**As a** developer,
**I want** the widget to be secure against XSS, CSRF, and data leakage,
**So that** it's safe to deploy on public contractor websites.

**Acceptance Criteria:**

**Given** FR-6.5 specifies security requirements
**When** I implement security measures
**Then** widget security includes:

**XSS Protection:**
- All user input sanitized before rendering (DOMPurify)
- React's built-in XSS protection via JSX
- No `dangerouslySetInnerHTML` usage
- Content Security Policy (CSP) compatible

**API Key Protection:**
- Tenant API key NEVER in client-side code
- Widget init exchanges API key for session token:
  ```typescript
  POST /api/v1/widget/init { api_key: "efofx_..." }
  Returns: { session_token: "uuid", branding: {...} }
  ```
- Session token used for all subsequent requests
- Session tokens expire after 24 hours

**HTTPS Enforcement:**
- Widget only loads over HTTPS
- API requests only to HTTPS endpoints
- Mixed content warnings prevented

**Rate Limiting:**
- Widget session: 50 messages max
- Same IP: 10 widget sessions per hour (prevents abuse)

**CORS Configuration:**
- Backend allows widget domain in CORS headers
- Credentials: include for session cookies

**And** security audit checklist passed:
- ✅ No sensitive data in localStorage/sessionStorage
- ✅ API key never exposed in network requests
- ✅ User input sanitized
- ✅ HTTPS only
- ✅ Rate limits prevent abuse

**Prerequisites:** Story 5.3 (chat UI implemented)

**Technical Notes:**
- Use DOMPurify for sanitizing user messages
- Implement session token rotation (optional)
- Monitor widget for suspicious activity (Sentry)
- Document security practices for tenant awareness
- Consider Subresource Integrity (SRI) for CDN-hosted widget

---

### Story 5.7: Create Widget Embed Code & Documentation

**As a** contractor,
**I want** simple copy-paste embed code for my website,
**So that** I can install the widget without technical expertise.

**Acceptance Criteria:**

**Given** FR-6.1 specifies <5 lines of code
**When** I create embed code
**Then** tenant dashboard displays:
```html
<!-- efOfX Estimation Widget -->
<script src="https://widget.efofx.ai/embed.js"></script>
<script>
  efofxWidget.init({ apiKey: 'efofx_abc123' });
</script>
```

**And** `embed.js` behavior:
- Loads asynchronously (doesn't block page load)
- Creates floating button (bottom-right corner, customizable position)
- Button shows unread message badge if applicable
- Clicking button opens chat overlay
- Overlay closable via X button or clicking outside
- Widget state persists across page navigation (session-based)

**And** installation documentation includes:
- Step-by-step guide with screenshots
- WordPress plugin installation (optional)
- Squarespace/Wix custom code instructions
- Troubleshooting common issues
- Testing checklist before going live

**And** when widget fails to load
**Then** graceful fallback: "Chat temporarily unavailable" message

**And** widget bundle size: <50KB gzipped (meets NFR-U1)

**Prerequisites:** Stories 5.1-5.6 (widget fully functional)

**Technical Notes:**
- Host `embed.js` on CDN (CloudFlare, DigitalOcean Spaces)
- Version embed.js: `/embed.js?v=1.0.0` for cache busting
- Provide test API key for demo purposes
- Create video tutorial for visual learners
- Track installation success rate (analytics)

---

### Story 5.8: Implement Widget Analytics & Monitoring

**As a** contractor,
**I want** to see widget performance metrics,
**So that** I know how many visitors engage and request estimates.

**Acceptance Criteria:**

**Given** tenants need visibility into widget effectiveness
**When** I implement analytics
**Then** widget tracks events:
- `widget_loaded` - Widget script loaded on page
- `widget_opened` - User clicked chat button
- `message_sent` - User sent message
- `estimate_generated` - Estimate displayed
- `lead_captured` - Email/phone captured
- `quote_requested` - User clicked "Request Quote"
- `widget_error` - Error occurred

**And** events sent to backend:
```typescript
POST /api/v1/widget/events
{
  session_id: "uuid",
  tenant_id: "tenant_abc",
  event_type: "estimate_generated",
  metadata: { project_type: "Pool", region: "SoCal" },
  timestamp: "2025-11-10T10:00:00Z"
}
```

**And** tenant dashboard displays metrics:
- Widget views (last 7/30 days)
- Engagement rate (opened / loaded)
- Estimate conversion rate (estimate / opened)
- Lead capture rate (lead / estimate)
- Quote request rate (quote / lead)
- Average conversation length (messages per session)

**And** funnel visualization:
```
100 Widget Opens
 ↓ 80% engagement
 80 Conversations Started
 ↓ 60% completion
 48 Estimates Generated
 ↓ 40% lead capture
 19 Leads Captured
 ↓ 50% quote request
 10 Quote Requests
```

**And** widget performance monitoring:
- Average load time (target <1s)
- Error rate (target <1%)
- API response times
- Alerts on degraded performance

**Prerequisites:** Story 5.7 (widget deployed)

**Technical Notes:**
- Use lightweight analytics (avoid Google Analytics bloat)
- Batch events (send every 30 seconds, not real-time)
- Respect Do Not Track (DNT) headers
- Store events for 90 days
- Dashboard built in tenant portal (post-MVP: separate story)

---

### Story 5.9: Manual QA Gate - Widget Integration Verification

**As a** QA tester,
**I want** a comprehensive test guide to verify the widget works perfectly on real contractor websites,
**So that** I can approve Epic 5 before production launch.

**Acceptance Criteria:**

**Given** Epic 5 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-5-widget-qa-guide.md` includes:

**Section 1: Prerequisites**
- Test tenant account with branding configured
- Three test websites: WordPress, static HTML, React SPA
- Mobile devices: iOS Safari, Android Chrome
- Desktop browsers: Chrome, Firefox, Safari, Edge
- Network throttling tools (simulate 3G)

**Section 2: Test Cases - Installation**
- **TC5.1:** Embed code on static HTML → widget loads within 1 second
- **TC5.2:** Embed on WordPress → widget doesn't conflict with theme
- **TC5.3:** Embed on React SPA → widget persists across route changes
- **TC5.4:** Multiple tabs → same session continues across tabs
- **TC5.5:** Invalid API key → graceful error message

**Section 3: Test Cases - Branding**
- **TC5.6:** Widget displays tenant logo (not efOfX)
- **TC5.7:** Primary color applied to buttons and accents
- **TC5.8:** Custom welcome message appears
- **TC5.9:** Button text matches tenant config
- **TC5.10:** Preview mode shows branding changes before publish

**Section 4: Test Cases - Conversational UI**
- **TC5.11:** User opens widget → welcome message displayed
- **TC5.12:** User types "pool" → bot asks clarifying questions
- **TC5.13:** Quick-select buttons work correctly
- **TC5.14:** Typing indicator shows while bot is thinking
- **TC5.15:** Chat history persists on page refresh (24 hours)
- **TC5.16:** Mobile keyboard doesn't obscure input field

**Section 5: Test Cases - Lead Capture**
- **TC5.17:** After sufficient info → lead form appears
- **TC5.18:** Email validation works (rejects invalid formats)
- **TC5.19:** Required fields enforced
- **TC5.20:** Privacy policy checkbox required (if GDPR applies)
- **TC5.21:** Lead submission sends email to contractor
- **TC5.22:** Duplicate lead (same email within 7 days) handled correctly

**Section 6: Test Cases - Estimate Display**
- **TC5.23:** Estimate displays P50 and P80 with clear labels
- **TC5.24:** Cost breakdown expands/collapses correctly
- **TC5.25:** LLM narrative is helpful and clear
- **TC5.26:** "Request Quote" sends email to contractor
- **TC5.27:** Estimate is mobile-responsive
- **TC5.28:** Estimate can be revisited in same session

**Section 7: Test Cases - Security**
- **TC5.29:** User input sanitized (test with `<script>alert('XSS')</script>`)
- **TC5.30:** API key never visible in network tab
- **TC5.31:** Session tokens expire after 24 hours
- **TC5.32:** Rate limit: 50 messages per session enforced
- **TC5.33:** HTTPS-only (widget doesn't load on HTTP)
- **TC5.34:** No mixed content warnings

**Section 8: Test Cases - Performance**
- **TC5.35:** Widget bundle size <50KB gzipped
- **TC5.36:** Widget loads in <1s on 3G connection
- **TC5.37:** Widget doesn't block host page rendering
- **TC5.38:** No memory leaks (test 30-minute session)
- **TC5.39:** Lighthouse score: Performance >90, Accessibility >90

**Section 9: Test Cases - Cross-Browser Compatibility**
- **TC5.40:** Chrome (desktop + mobile) → all features work
- **TC5.41:** Firefox → all features work
- **TC5.42:** Safari (iOS + macOS) → all features work
- **TC5.43:** Edge → all features work
- **TC5.44:** Older browsers (IE11) → graceful degradation message

**Section 10: Expected Results**
- Widget installs with <5 lines of code
- No efOfX branding visible
- Engagement rate >10% (opened/loaded)
- Lead capture rate >30% (leads/estimates)
- Zero XSS vulnerabilities
- Works on 95% of modern browsers

**Section 11: Known Limitations**
- No multi-language support yet
- No voice input yet
- No offline mode
- Widget analytics dashboard not built yet (data collected only)

**Prerequisites:** Stories 5.1-5.8

**Technical Notes:**
- Test on real contractor websites before launch
- Use BrowserStack for cross-browser testing
- Monitor Sentry for widget errors during QA
- Create video recordings of successful flows
- Get feedback from 3 non-technical users

---

## Epic 6: Feedback & Calibration System

**Goal:** Close the learning loop by capturing actual project outcomes from customers and contractors, calculating calibration metrics, and using feedback to improve future estimates. This is the "self-improving" breakthrough that separates efOfX from competitors.

**Value Proposition:** Transforms efOfX from a static estimation tool into a learning system that gets more accurate over time. Builds trust through visible calibration metrics.

---

### Story 6.1: Create Customer Feedback Magic Link System

**As a** customer,
**I want** to submit actual project outcomes via email link (no login required),
**So that** I can help improve estimates for future projects.

**Acceptance Criteria:**

**Given** FR-5.1 specifies magic link feedback
**When** I implement customer feedback
**Then** after estimate is generated
**And** lead email captured
**Then** system sends follow-up email 90 days later:
```
Subject: How did your {project_type} project go?

Hi {name},

90 days ago, we estimated your pool installation at $75k-$92k.
We'd love to know how it actually turned out!

[Share Your Actual Costs] (magic link)

This helps us improve estimates for future projects.
Thanks!
- {Contractor Name}
```

**And** magic link format: `https://efofx.ai/feedback/{token}`
- Token is JWT with: estimate_id, email, expires_at (90 days)
- Token is single-use (invalidated after submission)

**And** feedback form at magic link includes:
- Project summary (read-only reminder)
- Original estimate range (P50/P80)
- Actual final cost (required, validated)
- Actual timeline in weeks (required, validated)
- Accuracy rating (1-5 stars)
- What changed from estimate? (textarea, optional)
- Would you recommend this contractor? (yes/no)

**And** `POST /api/v1/feedback/customer` endpoint:
- Validates magic link token
- Saves feedback to `feedback` collection
- Calculates variance: `(actual - p50) / p50 * 100`
- Invalidates token (prevent re-submission)
- Sends thank-you email to customer
- Notifies contractor of feedback submission

**Prerequisites:** Story 4.3 (estimates include email), Story 5.4 (lead capture)

**Technical Notes:**
- Use SendGrid scheduled sends (90-day delay)
- Magic link expires after 90 days from email send
- Form is mobile-friendly (60% will use mobile)
- Track email open rate and link click rate
- A/B test email timing (60 days vs 90 days vs 120 days)

---

### Story 6.2: Create Contractor Feedback Dashboard

**As a** contractor (tenant),
**I want** to submit actual project outcomes and explain discrepancies,
**So that** the system learns from my real-world experience.

**Acceptance Criteria:**

**Given** FR-5.2 specifies contractor feedback
**When** I implement contractor feedback interface
**Then** tenant dashboard shows "Estimates Awaiting Feedback" list:
- All estimates created in last 180 days
- Not yet marked complete
- Filterable by project type, date range
- Sortable by estimate date

**And** clicking estimate opens feedback form:
- Original estimate summary (read-only)
- Customer feedback (if submitted, read-only)
- Actual cost breakdown by category (materials, labor, etc.)
- Actual timeline (start date, end date)
- Discrepancy explanation (textarea)
- Flags: scope_creep, hidden_costs, market_changes, estimate_too_high, estimate_too_low

**And** `POST /api/v1/feedback/contractor` endpoint:
- Requires JWT authentication (tenant)
- Saves detailed breakdown to `feedback` collection
- Links to customer feedback if exists
- Flags discrepancies >20% for review
- Updates estimate status to "complete"

**And** when customer and contractor feedback both exist
**And** variance between them >10%
**Then** system creates review task for admin:
```
Discrepancy Alert:
Customer reported: $82,500
Contractor reported: $89,000
Variance: 7.9%
Estimate was: $75,000 (P50)
```

**Prerequisites:** Story 6.1 (customer feedback system)

**Technical Notes:**
- Contractor feedback optional but encouraged (gamify: "80% feedback rate!")
- Allow partial feedback (cost only, timeline only)
- Auto-fill cost breakdown from original estimate as starting point
- Track contractor feedback rate per tenant
- Highlight outliers (>30% variance) for investigation

---

### Story 6.3: Calculate Calibration Metrics

**As a** system,
**I want** to calculate accuracy metrics from feedback data,
**So that** tenants can see how well estimates match reality.

**Acceptance Criteria:**

**Given** FR-5.3 specifies calibration metrics
**When** I implement metrics calculation
**Then** `app/services/calibration_service.py` calculates:

**Per-Tenant Metrics:**
- Total estimates submitted
- Feedback submission count
- Feedback rate (submissions / total)
- Average cost variance % (mean of all `(actual - p50) / p50`)
- Average timeline variance %
- Percentage within 20% of P50 (success metric)
- Percentage within P50-P80 range (calibration quality)

**Per-Reference-Class Metrics:**
- Count of estimates using this class
- Feedback count for this class
- Average variance for this class
- Trend: improving, stable, or degrading

**Calculation Triggers:**
- Real-time: Immediately after feedback submission
- Batch: Nightly for all tenants (updates historical trends)
- On-demand: When tenant views calibration dashboard

**And** `GET /api/v1/calibration/metrics` endpoint returns:
```json
{
  "tenant_id": "abc",
  "period": "last_90_days",
  "total_estimates": 47,
  "feedback_count": 21,
  "feedback_rate": 0.45,
  "cost_metrics": {
    "avg_variance_pct": 8.2,
    "median_variance_pct": 6.5,
    "within_20_pct": 0.71,
    "within_p50_p80_range": 0.81
  },
  "by_reference_class": {
    "Pool - Midrange": {
      "count": 12,
      "avg_variance": 6.5,
      "trend": "improving"
    }
  }
}
```

**And** metrics are cached (1 hour TTL) for performance

**And** historical trend data stored for graphing (last 12 months)

**Prerequisites:** Stories 6.1-6.2 (feedback data exists)

**Technical Notes:**
- Use MongoDB aggregation pipeline for efficient calculation
- Handle edge cases: zero feedback, all estimates too high/low
- Calculate confidence intervals for small sample sizes (<10 feedback)
- Track "accuracy improving over time" as key metric
- Alert tenant if accuracy degrading (>5% worse month-over-month)

---

### Story 6.4: Display Calibration Dashboard for Tenants

**As a** contractor (tenant),
**I want** to see how accurate my estimates are over time,
**So that** I can build trust with customers and improve my quoting.

**Acceptance Criteria:**

**Given** calibration metrics are calculated
**When** I implement the dashboard UI
**Then** tenant portal includes "Calibration Metrics" page with:

**Summary Cards:**
- Feedback Rate: "45% (21 of 47 estimates)" with progress bar
- Accuracy: "71% within 20% of estimate" with trend arrow (↑ improving)
- Average Variance: "+8.2% (estimates trending slightly high)"
- Trust Score: "92/100" (composite of accuracy, feedback rate, transparency)

**Charts:**
- Line chart: Accuracy over time (last 12 months)
- Bar chart: Variance by reference class (pool, ADU, kitchen, etc.)
- Distribution: Histogram of actual costs vs estimated P50/P80 ranges

**Insights:**
- "Your pool estimates are most accurate (6.5% average variance)"
- "ADU estimates trending 15% high - consider adjusting complexity factors"
- "Feedback rate increased 12% this month - great job!"

**Public Trust Badge:**
- Embeddable badge for contractor website:
  ```html
  <div class="efofx-trust-badge">
    <img src="https://efofx.ai/badge/abc123.svg" alt="Verified Estimate Accuracy: 71%">
  </div>
  ```
- Badge updates monthly with latest calibration metrics
- Clicking badge links to public calibration page (optional, tenant can enable)

**And** calibration data exportable as CSV for tenant analysis

**Prerequisites:** Story 6.3 (metrics calculated)

**Technical Notes:**
- Use Chart.js or Recharts for visualizations
- Trust Score algorithm: `(within_20_pct * 0.6 + feedback_rate * 0.3 + trend * 0.1) * 100`
- Public badge requires tenant opt-in (privacy)
- Benchmark against industry standards (if available)
- Highlight improving trends to encourage feedback collection

---

### Story 6.5: Implement Synthetic Data Validation & Tuning

**As a** system,
**I want** to compare synthetic reference classes to real feedback data,
**So that** synthetic data stays calibrated and accurate.

**Acceptance Criteria:**

**Given** FR-2.3 specifies synthetic data validation
**When** I implement validation
**Then** nightly batch job runs `app/jobs/validate_synthetic_data.py`:

**For each synthetic reference class with >=5 feedback submissions:**
- Calculate average actual cost vs synthetic P50/P80
- Calculate variance: `(avg_actual - synthetic_p50) / synthetic_p50 * 100`
- Flag if variance >25% (needs tuning)
- Generate tuning recommendation:
  ```json
  {
    "reference_class_id": "pool_midrange_socal",
    "current_p50": 75000,
    "avg_actual": 82000,
    "variance_pct": 9.3,
    "recommendation": "Increase P50 by 7% to $80,250",
    "confidence": "high (12 data points)",
    "status": "pending_review"
  }
  ```

**And** `GET /admin/synthetic/validate` endpoint shows:
- List of all reference classes
- Validation status: validated, needs_tuning, insufficient_data
- Pending tuning recommendations
- Approval workflow for admin

**And** when admin approves tuning
**Then** reference class updated with new P50/P80 values
**And** change logged in audit trail:
  ```
  2025-11-15: Pool - Midrange (SoCal) P50 updated: $75k → $80k
  Reason: 12 feedback submissions averaged $82k (9.3% high)
  Approved by: admin@efofx.ai
  ```

**And** future estimates use updated values immediately

**And** validation report emailed to admin weekly (summary of tuning activity)

**Prerequisites:** Story 6.3 (feedback data aggregated)

**Technical Notes:**
- Require minimum 5 feedback submissions before tuning (statistical significance)
- Use median instead of mean (resistant to outliers)
- Track how many estimates used each reference class version (for rollback)
- Consider seasonal adjustments (construction costs vary by quarter)
- Post-MVP: Automate tuning for well-calibrated classes (>20 data points)

---

### Story 6.6: Implement LLM Prompt Refinement from Feedback

**As a** system,
**I want** to improve LLM narrative prompts based on feedback comments,
**So that** assumptions and risks become more accurate over time.

**Acceptance Criteria:**

**Given** feedback includes qualitative comments ("soil was harder than expected")
**When** I implement prompt refinement
**Then** monthly review process:

**Step 1: Analyze Feedback Comments**
- Extract common themes from feedback comments (NLP or manual review)
- Identify missed assumptions (e.g., "rock layer" mentioned in 5 pool projects)
- Identify missed risks (e.g., "permit delays" in 8 ADU projects)

**Step 2: Generate Prompt Improvements**
- Use LLM to suggest prompt enhancements:
  ```
  Analysis: 5 pool feedback comments mentioned "unexpected rock/hardpan soil"
  Current prompt: Assumes standard soil conditions
  Suggested improvement: Add risk factor - "Rock or hardpan soil could add $5-15k"
  ```

**Step 3: Create Prompt Version**
- Copy current prompt JSON → new version (1.0.0 → 1.1.0)
- Apply improvements
- Commit to git with detailed changelog
- A/B test: 50% estimates use v1.0.0, 50% use v1.1.0

**Step 4: Measure Impact**
- After 30 days, compare feedback for v1.0.0 vs v1.1.0 estimates
- Metrics: accuracy rating, "helpful" comments, variance
- If v1.1.0 performs better (>5% improvement) → promote to 100%
- If not → rollback or iterate

**And** `GET /admin/prompts/performance` shows:
```json
{
  "estimate_narrative": {
    "v1.0.0": { "avg_rating": 4.2, "count": 150 },
    "v1.1.0": { "avg_rating": 4.5, "count": 148 },
    "recommendation": "Promote v1.1.0 to production"
  }
}
```

**Prerequisites:** Story 4.2 (prompt versioning), Story 6.1 (feedback comments collected)

**Technical Notes:**
- Start with manual review (MVP), automate with NLP post-MVP
- Use LLM (GPT-4) to analyze feedback themes (meta!)
- Prompt improvements require code review (prompts are code)
- Track prompt version performance in calibration metrics
- Consider domain-specific prompts (construction vs IT/dev)

---

### Story 6.7: Manual QA Gate - Feedback Loop Verification

**As a** QA tester,
**I want** a test guide to verify the feedback system closes the loop and improves accuracy,
**So that** I can confirm Epic 6 delivers the self-improving promise.

**Acceptance Criteria:**

**Given** Epic 6 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-6-feedback-qa-guide.md` includes:

**Section 1: Prerequisites**
- Test tenant with 20+ estimates created
- Customer email addresses for testing magic links
- Contractor login credentials
- Admin access for validation dashboard

**Section 2: Test Cases - Customer Feedback**
- **TC6.1:** Magic link email sent 90 days after estimate
- **TC6.2:** Magic link opens feedback form (no login required)
- **TC6.3:** Form shows original estimate summary
- **TC6.4:** Customer submits actual cost → variance calculated correctly
- **TC6.5:** Magic link invalidated after submission (can't resubmit)
- **TC6.6:** Expired magic link (>90 days) → helpful error message
- **TC6.7:** Thank-you email sent to customer after submission

**Section 3: Test Cases - Contractor Feedback**
- **TC6.8:** Contractor dashboard shows "Estimates Awaiting Feedback"
- **TC6.9:** Contractor submits detailed cost breakdown
- **TC6.10:** Discrepancy flags saved correctly (scope_creep, hidden_costs)
- **TC6.11:** Customer + contractor feedback both exist → variance calculated
- **TC6.12:** Large discrepancy (>20%) → flagged for admin review
- **TC6.13:** Estimate status updated to "complete"

**Section 4: Test Cases - Calibration Metrics**
- **TC6.14:** Metrics calculated immediately after feedback submission
- **TC6.15:** Feedback rate displayed correctly (21/47 = 44.7%)
- **TC6.16:** Average variance % matches manual calculation
- **TC6.17:** "Within 20%" percentage calculated correctly
- **TC6.18:** Per-reference-class metrics accurate
- **TC6.19:** Trend detection works (improving/stable/degrading)
- **TC6.20:** Metrics cached (subsequent calls <50ms)

**Section 5: Test Cases - Calibration Dashboard**
- **TC6.21:** Summary cards display current metrics
- **TC6.22:** Charts render correctly (line, bar, histogram)
- **TC6.23:** Insights are helpful and actionable
- **TC6.24:** Trust badge embeddable and updates monthly
- **TC6.25:** CSV export includes all expected columns

**Section 6: Test Cases - Synthetic Data Validation**
- **TC6.26:** Validation job runs nightly (check cron logs)
- **TC6.27:** Reference class with >5 feedback → variance calculated
- **TC6.28:** Variance >25% → tuning recommendation generated
- **TC6.29:** Admin approves tuning → reference class P50/P80 updated
- **TC6.30:** Updated values used in new estimates immediately
- **TC6.31:** Audit log tracks tuning changes with reason

**Section 7: Test Cases - Prompt Refinement**
- **TC6.32:** Feedback comments analyzed for common themes
- **TC6.33:** Prompt improvement suggestions generated
- **TC6.34:** New prompt version created (1.0.0 → 1.1.0)
- **TC6.35:** A/B test: estimates use both versions
- **TC6.36:** Performance comparison after 30 days
- **TC6.37:** Better-performing version promoted to 100%

**Section 8: Expected Results**
- Magic links work reliably (>95% deliverability)
- Feedback submission rate >30% (aspirational: 40%)
- Calibration metrics accurate to 2 decimal places
- Synthetic data tuning improves accuracy by 10%+ after 3 months
- Prompt refinement increases "helpful" ratings by 5%+

**Section 9: Known Limitations**
- Manual prompt review (no automated NLP yet)
- Simple trend detection (no forecasting)
- No multi-language feedback forms
- Trust badge doesn't update in real-time (monthly cache)

**Prerequisites:** Stories 6.1-6.6

**Technical Notes:**
- Test full feedback loop with real emails (not just localhost)
- Manually verify calibration math with spreadsheet
- Test with edge cases: zero feedback, all too high, all too low
- Monitor email deliverability (SendGrid dashboard)
- Review 10+ feedback submissions for UX friction points

---

## Epic 7: Code Consolidation & Refactoring

**Goal:** Apply DRY (Don't Repeat Yourself) and YAGNI (You Aren't Gonna Need It) principles across the codebase now that all features are implemented. Create shared libraries, eliminate duplication, remove unused code, and establish consistent patterns for maintainability.

**Value Proposition:** Clean, maintainable codebase that's easier for future development. Reduces technical debt before it accumulates. Establishes best practices and patterns.

---

### Story 7.1: Identify and Extract Shared Backend Utilities

**As a** developer,
**I want** common backend utilities consolidated into shared modules,
**So that** code isn't duplicated and changes propagate consistently.

**Acceptance Criteria:**

**Given** Epics 1-6 are complete
**When** I audit the codebase for duplication
**Then** identify and extract to `app/utils/`:

**common.py:**
- `format_currency(amount, currency="USD")` - Used across estimates, feedback, calibration
- `calculate_variance_pct(actual, estimate)` - Used in feedback and metrics
- `format_date_range(start, end)` - Used in estimates and dashboard
- `sanitize_input(text)` - Used in chat and feedback forms

**validators.py:**
- `validate_email(email)` - Used in tenant registration, lead capture, feedback
- `validate_phone(phone)` - Used in lead capture
- `validate_api_key_format(key)` - Used in tenant management
- `validate_cost_range(min, max)` - Used in estimates and reference classes

**mongodb_helpers.py:**
- `get_tenant_filter(tenant_id, **extra_filters)` - Used in ALL tenant-scoped queries
- `paginate_query(query, page, page_size)` - Used in dashboard lists
- `ensure_indexes(collection, indexes)` - Used in migrations

**And** update all existing code to use shared utilities instead of inline logic

**And** remove duplicated code (DRY compliance)

**And** add unit tests for all shared utilities (>90% coverage)

**And** document each utility with docstrings and usage examples

**Prerequisites:** Epics 1-6 complete

**Technical Notes:**
- Use ripgrep/grep to find duplicate patterns
- Maintain backward compatibility (don't break existing code)
- Consider edge cases in shared functions
- Shared code must have zero dependencies on specific features

---

### Story 7.2: Create Shared Frontend Components Library

**As a** developer,
**I want** common React components extracted to a shared library,
**So that** widget UI is consistent and reusable.

**Acceptance Criteria:**

**Given** widget is fully implemented (Epic 5)
**When** I extract shared components
**Then** create `apps/efofx-widget/src/components/shared/`:

**Button.tsx:**
- Primary, secondary, tertiary variants
- Loading state, disabled state
- Icon support
- Consistent sizing and styling
- Used in: chat input, lead form, estimate display

**Input.tsx:**
- Text, email, phone, number variants
- Validation state display (error, success)
- Label and help text support
- Used in: lead capture, feedback forms

**Card.tsx:**
- Container with consistent padding/spacing
- Header, body, footer sections
- Used in: estimate display, calibration dashboard

**LoadingSpinner.tsx:**
- Branded spinner with primary color
- Size variants (small, medium, large)
- Used in: API loading states throughout widget

**Modal.tsx:**
- Overlay with close button
- Accessible (focus trap, ESC to close)
- Used in: lead capture, estimate display

**And** refactor existing components to use shared library

**And** create Storybook documentation for all shared components (optional for MVP)

**And** ensure all shared components have:
- TypeScript types
- Accessibility (ARIA labels)
- Mobile-responsive
- Brand color theming

**Prerequisites:** Epic 5 complete (widget implemented)

**Technical Notes:**
- Consider headless UI libraries (Radix, Headless UI) for accessibility
- Use Tailwind's @apply directive for consistent styling
- Shared components should be pure (no business logic)
- Test shared components in isolation

---

### Story 7.3: Apply YAGNI - Remove Unused Code and Features

**As a** developer,
**I want** to identify and remove unused code, commented-out sections, and over-engineered features,
**So that** the codebase stays lean and maintainable.

**Acceptance Criteria:**

**Given** Epics 1-6 included some speculative code
**When** I audit for unused code
**Then** identify and remove:

**Backend:**
- Unused imports (run `autoflake` or similar)
- Commented-out code blocks (manual review)
- Unused API endpoints (check access logs)
- Over-engineered abstractions (e.g., generic factories with single implementation)
- Dead code (unreachable branches, unused functions)

**Frontend:**
- Unused React components (check imports)
- Unused CSS classes (run PurgeCSS)
- Unused npm dependencies (run `depcheck`)
- Commented-out console.log statements

**Configuration:**
- Unused environment variables
- Unused feature flags
- Unused database indexes (check MongoDB slow query log)

**And** document removal decisions:
```markdown
# Removed Code Audit (2025-11-10)
- Removed `app/utils/legacy_estimator.py` - Unused since RCF engine implemented
- Removed `apps/widget/src/components/OldChat.tsx` - Replaced by new conversational UI
- Removed env var `LEGACY_API_KEY` - No longer referenced anywhere
```

**And** run tests to confirm nothing broke after removal

**And** reduce overall codebase size by 10-15%

**Prerequisites:** Epics 1-6 complete

**Technical Notes:**
- Use coverage tools to find untested (potentially unused) code
- Be conservative - if unsure, leave it
- Comment out first, remove after 1 sprint with no issues
- Check git history to understand why code was added before removing

---

### Story 7.4: Establish Code Quality Standards & Documentation

**As a** development team,
**I want** documented coding standards and architecture patterns,
**So that** future development maintains consistency.

**Acceptance Criteria:**

**Given** the codebase is refactored and consolidated
**When** I create documentation
**Then** `docs/CONTRIBUTING.md` includes:

**Code Style:**
- Python: Black formatting, Flake8 linting, type hints required
- TypeScript: ESLint + Prettier, strict mode enabled
- Commit messages: Conventional Commits format
- Branch naming: `feature/story-X-Y-short-description`

**Architecture Patterns:**
- Backend: Service layer pattern (controllers → services → repositories)
- Frontend: Component composition, custom hooks for logic
- API: RESTful conventions, versioned endpoints (/api/v1/...)
- Database: Tenant-scoped queries ALWAYS include tenant_id

**Testing Standards:**
- Unit tests: >70% coverage for services and utilities
- Integration tests: All API endpoints have happy path + error case tests
- E2E tests: Critical user flows (widget install, estimate generation, feedback submission)
- Test naming: `test_<function>_<scenario>_<expected_result>`

**Pull Request Checklist:**
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No commented-out code
- [ ] Tenant isolation verified (if applicable)
- [ ] Security reviewed (if user input involved)
- [ ] Accessibility checked (if UI change)

**And** `docs/ARCHITECTURE.md` updated with:
- System overview diagram
- Data flow diagrams (estimate generation, feedback loop)
- Security architecture (BYOK, tenant isolation)
- Deployment architecture (DO App Platform + MongoDB Atlas)

**And** inline code documentation:
- All public functions have docstrings
- Complex algorithms have explanatory comments
- Configuration files have helpful comments

**Prerequisites:** Stories 7.1-7.3 complete

**Technical Notes:**
- Use ADRs (Architecture Decision Records) for major decisions
- Keep documentation close to code (in repo, not external wiki)
- Automate what can be automated (linting, formatting, testing)
- Review and update docs quarterly

---

### Story 7.5: Manual QA Gate - Code Quality & Maintainability Verification

**As a** QA tester and future developer,
**I want** a comprehensive code quality checklist,
**So that** I can verify the codebase is production-ready and maintainable.

**Acceptance Criteria:**

**Given** Epic 7 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-7-code-quality-qa-guide.md` includes:

**Section 1: Prerequisites**
- Full codebase access (git clone)
- Development environment set up
- All Epics 1-6 test guides passing
- Code review tools (linters, formatters)

**Section 2: Test Cases - Code Organization**
- **TC7.1:** Backend follows service layer pattern (controllers → services → DB)
- **TC7.2:** Frontend components organized by feature/shared
- **TC7.3:** No circular dependencies (run `madge` or similar)
- **TC7.4:** Folder structure matches architecture doc
- **TC7.5:** Related code grouped logically (high cohesion)

**Section 3: Test Cases - DRY Compliance**
- **TC7.6:** No duplicate utility functions (search for similar patterns)
- **TC7.7:** Shared components used consistently (no inline duplicates)
- **TC7.8:** Database query helpers used everywhere (no raw queries)
- **TC7.9:** Validation logic centralized (not scattered)
- **TC7.10:** Error handling patterns consistent

**Section 4: Test Cases - YAGNI Compliance**
- **TC7.11:** No unused imports (run `autoflake`, `depcheck`)
- **TC7.12:** No commented-out code blocks
- **TC7.13:** No unused API endpoints (check routes file)
- **TC7.14:** No unused database collections
- **TC7.15:** No over-engineered abstractions (unnecessary interfaces)

**Section 5: Test Cases - Code Quality**
- **TC7.16:** Python code passes Black formatting
- **TC7.17:** Python code passes Flake8 linting (zero errors)
- **TC7.18:** TypeScript code passes ESLint (zero errors)
- **TC7.19:** All functions have type hints (Python) or types (TypeScript)
- **TC7.20:** No `any` types in TypeScript (use proper types)
- **TC7.21:** No security vulnerabilities (run `safety`, `npm audit`)

**Section 6: Test Cases - Testing**
- **TC7.22:** Backend test coverage >70% (run `pytest --cov`)
- **TC7.23:** All API endpoints have tests (happy path + errors)
- **TC7.24:** Shared utilities have unit tests (>90% coverage)
- **TC7.25:** Widget components have tests (React Testing Library)
- **TC7.26:** E2E tests pass (widget install, estimate, feedback flows)
- **TC7.27:** Tests run in <5 minutes (CI/CD feasible)

**Section 7: Test Cases - Documentation**
- **TC7.28:** README.md has setup instructions
- **TC7.29:** CONTRIBUTING.md defines code standards
- **TC7.30:** ARCHITECTURE.md up-to-date with current system
- **TC7.31:** All public functions have docstrings
- **TC7.32:** API endpoints documented (Swagger/OpenAPI)
- **TC7.33:** Environment variables documented (.env.example)

**Section 8: Test Cases - Performance**
- **TC7.34:** No N+1 database queries (check slow query log)
- **TC7.35:** Database queries use indexes (explain plans)
- **TC7.36:** Frontend bundle size <50KB gzipped
- **TC7.37:** API response times <500ms (p95, excluding LLM)
- **TC7.38:** No memory leaks (run profiler on long sessions)

**Section 9: Test Cases - Security**
- **TC7.39:** All tenant queries include tenant_id filter
- **TC7.40:** No sensitive data in logs (API keys, passwords)
- **TC7.41:** No SQL/NoSQL injection vulnerabilities
- **TC7.42:** No XSS vulnerabilities (user input sanitized)
- **TC7.43:** Dependencies up-to-date (no critical CVEs)
- **TC7.44:** HTTPS enforced everywhere

**Section 10: Expected Results**
- Codebase is 10-15% smaller after YAGNI cleanup
- Zero linting errors
- >70% test coverage
- All documentation current
- Code follows consistent patterns
- New developers can onboard in <1 day

**Section 11: Maintainability Score**
Calculate composite score:
- Code coverage >70%: 25 points
- Zero linting errors: 20 points
- Documentation complete: 20 points
- No code duplication: 15 points
- Tests run <5min: 10 points
- No security vulnerabilities: 10 points

**Target: 90+/100 for production-ready**

**Prerequisites:** Stories 7.1-7.4

**Technical Notes:**
- Use automated tools where possible (linters, coverage, security scanners)
- Manual code review of complex logic (RCF engine, calibration, BYOK)
- Have a non-original developer attempt to understand and modify code
- Document any technical debt that remains (with rationale for deferring)

---

_End of Epic Breakdown_

---

## Next Steps

### Immediate Actions

1. **Review This Epic Breakdown**: Ensure all 44-51 stories align with PRD requirements
2. **Validate Story Sequencing**: Confirm no forward dependencies exist
3. **Estimate Story Points** (optional): Assign complexity/effort to each story for sprint planning

### Workflow Progression

After epic approval, proceed to:

**Option 1: Architecture Workflow**
Run `/bmad:bmm:workflows:architecture` to create technical specifications for each epic.

**Option 2: Sprint Planning Workflow**
Run `/bmad:bmm:workflows:sprint-planning` to generate sprint status tracking file and begin Phase 4 implementation.

**Option 3: Story Creation Workflow**
Run `/bmad:bmm:workflows:create-story` to generate individual story markdown files for development.

---

_This epic breakdown transforms the efOfX PRD into 44-51 bite-sized, implementable stories across 7 sequential epics. Each story is vertically sliced, independently valuable, and sized for single dev agent completion. The structure enables incremental value delivery while maintaining quality through QA gates at each epic boundary._
