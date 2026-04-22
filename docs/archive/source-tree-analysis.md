# Source Tree Analysis - Efofx Workspace

**Repository Type:** Monorepo
**Active Parts:** 2 (efofx-estimate, estimator-mcp-functions)
**Reference Architecture:** estimator-project (newer patterns for future migration)

## Project Root Structure

```
efofx-workspace/
├── apps/                              # Application monorepo
│   ├── efofx-estimate/               # ⭐ PRIMARY BACKEND (Python/FastAPI)
│   ├── estimator-mcp-functions/      # ⭐ SERVERLESS FUNCTIONS (Node.js)
│   ├── estimator-project/            # 📚 Reference architecture (newer patterns)
│   └── scripts/                      # Utility scripts
├── bmad/                             # BMad Method framework (workflow automation)
└── docs/                             # Generated documentation

```

---

## Part 1: efofx-estimate (Primary Backend)

**Location:** `apps/efofx-estimate/`
**Type:** Python/FastAPI Backend Service
**Entry Point:** `app/main.py`
**Database:** MongoDB (Motor async driver)
**AI/LLM:** OpenAI GPT integration

### Directory Structure

```
apps/efofx-estimate/
├── app/                              # Main application package
│   ├── main.py                       # 🚪 FastAPI entry point, middleware setup
│   ├── api/                          # API layer (routes and endpoints)
│   │   ├── routes.py                 # All API endpoints defined here
│   │   │                             # - /estimate/* (estimation)
│   │   │                             # - /chat/* (conversational scoping)
│   │   │                             # - /feedback/* (outcome tracking)
│   │   │                             # - /status (health check)
│   │   └── __init__.py
│   ├── core/                         # Core system functionality
│   │   ├── config.py                 # Application settings (Pydantic Settings)
│   │   ├── constants.py              # Enums, status codes, API messages
│   │   ├── security.py               # JWT auth, tenant validation, rate limiting
│   │   └── logging.py                # Structured logging configuration
│   ├── db/                           # Database layer
│   │   ├── mongodb.py                # 🔌 MongoDB connection manager
│   │   │                             # - connect_to_mongo()
│   │   │                             # - get_database()
│   │   │                             # - health_check()
│   │   └── repositories/             # Data access patterns (if exists)
│   ├── models/                       # Pydantic data models
│   │   ├── tenant.py                 # Multi-tenant data model
│   │   ├── estimation.py             # EstimationRequest, EstimationResult
│   │   ├── reference.py              # ReferenceClass, ReferenceProject
│   │   ├── chat.py                   # ChatRequest, ChatResponse
│   │   ├── feedback.py               # FeedbackCreate, FeedbackSummary
│   │   └── __init__.py
│   ├── services/                     # Business logic layer
│   │   ├── estimation_service.py     # 🧠 Core estimation logic
│   │   │                             # - start_estimation()
│   │   │                             # - get_estimation()
│   │   │                             # - upload_image()
│   │   ├── chat_service.py           # Conversational estimation flow
│   │   │                             # - send_message()
│   │   │                             # - get_chat_history()
│   │   ├── feedback_service.py       # Outcome tracking & calibration
│   │   │                             # - submit_feedback()
│   │   │                             # - get_feedback_summary()
│   │   ├── rcf_service.py            # Reference Class Forecasting engine
│   │   │                             # - find_best_reference_class()
│   │   │                             # - apply_adjustments()
│   │   ├── openai_service.py         # OpenAI API integration
│   │   │                             # - generate_estimate_narrative()
│   │   │                             # - extract_project_details()
│   │   ├── tenant_service.py         # Tenant management
│   │   └── __init__.py
│   ├── utils/                        # Utility functions
│   │   ├── validators.py             # Custom Pydantic validators
│   │   ├── helpers.py                # General helper functions
│   │   └── __init__.py
│   └── README.md                     # Application structure documentation
├── tests/                            # Test suite
│   ├── test_estimation.py            # Estimation endpoint tests
│   ├── test_chat.py                  # Chat endpoint tests
│   ├── test_rcf.py                   # RCF logic tests
│   ├── test_models.py                # Pydantic model tests
│   ├── conftest.py                   # Pytest fixtures
│   └── README.md                     # Testing documentation
├── scripts/                          # Utility scripts
│   ├── seed_reference_classes.py     # Populate reference class data
│   ├── migrate_db.py                 # Database migration scripts
│   └── README.md
├── .env                              # Environment variables (not in git)
├── requirements.txt                  # Python dependencies
├── pyproject.toml                    # Project metadata & build config
├── pytest.ini                        # Pytest configuration
├── .gitignore                        # Git ignore patterns
└── README.md                         # 📖 Project documentation

```

### Critical Paths

**Entry Points:**
- `app/main.py` - FastAPI application initialization

**API Layer:**
- `app/api/routes.py` - All HTTP endpoints

**Business Logic:**
- `app/services/estimation_service.py` - Core estimation engine
- `app/services/rcf_service.py` - Reference Class Forecasting
- `app/services/openai_service.py` - LLM integration

**Data Layer:**
- `app/db/mongodb.py` - Database connections
- `app/models/` - All Pydantic models for validation

**Security:**
- `app/core/security.py` - JWT validation, tenant auth, rate limiting

---

## Part 2: estimator-mcp-functions (Serverless)

**Location:** `apps/estimator-mcp-functions/`
**Type:** DigitalOcean Serverless Functions (Node.js)
**Runtime:** Node.js 18+ (ESM modules)
**Deployment:** DigitalOcean App Platform

### Directory Structure

```
apps/estimator-mcp-functions/
├── packages/                         # Function packages
│   └── estimator/                    # Estimator MCP namespace
│       ├── manifest/                 # 📋 MCP manifest endpoint
│       │   ├── index.js              # Returns available functions
│       │   └── package.json
│       ├── reference_classes-get/    # 🔍 Get reference class by ID
│       │   ├── index.js              # Fetch single reference class
│       │   └── package.json
│       ├── reference_classes-query/  # 🔎 Query reference classes
│       │   ├── index.js              # Search with keyword/region filters
│       │   └── package.json
│       └── adjustments-apply/        # ⚙️ Apply regional adjustments
│           ├── index.js              # Calculate adjusted estimates
│           └── package.json
├── lib/                              # Shared libraries
│   ├── log.js                        # Pino structured logging
│   ├── db.js                         # MongoDB connection helpers
│   ├── auth.js                       # JWT validation
│   └── cache.js                      # LRU cache for reference classes
├── package.json                      # Root package.json
├── .env                              # Environment variables (not in git)
├── DEPLOYMENT.md                     # 📖 Deployment instructions
└── README.md                         # Project documentation

```

### Function Details

**manifest/**
- **Purpose:** Returns MCP server manifest with available functions
- **HTTP Method:** GET
- **Auth:** Public
- **Cache:** 10 minutes

**reference_classes-get/**
- **Purpose:** Fetch single reference class by MongoDB ObjectId
- **HTTP Method:** GET
- **Auth:** JWT required
- **Cache:** 5 minutes (LRU)

**reference_classes-query/**
- **Purpose:** Search reference classes by keywords, category, region
- **HTTP Method:** POST
- **Auth:** JWT required
- **Cache:** None (dynamic queries)

**adjustments-apply/**
- **Purpose:** Apply regional/complexity factors to baseline estimates
- **HTTP Method:** POST
- **Auth:** JWT required
- **Cache:** None (per-request calculations)

### Shared Libraries (`lib/`)

**log.js** - Structured logging with Pino
- `createRequestLogger(traceId, userId)` - Per-request logger
- JSON-formatted logs for easy parsing

**db.js** - MongoDB helpers
- Connection pooling for serverless
- Singleton pattern for warm starts

**auth.js** - JWT validation
- `verifyToken(token)` - Validate and decode JWT
- Tenant extraction from claims

**cache.js** - LRU caching
- Reference class caching (5min TTL)
- Reduces MongoDB reads by ~80%

---

## Part 3: estimator-project (Reference Architecture)

**Location:** `apps/estimator-project/`
**Status:** Reference/newer architectural patterns
**Note:** Contains better structure (RCF module, observability) but less implementation

### Notable Patterns to Migrate

```
apps/estimator-project/app/
├── rcf/                              # 📚 Dedicated RCF module (good pattern!)
│   ├── matcher.py                    # Reference class matching
│   ├── adjuster.py                   # Regional adjustments
│   └── forecaster.py                 # Forecast generation
├── observability/                    # 📊 Structured observability (good pattern!)
│   ├── metrics.py                    # Prometheus metrics
│   ├── tracing.py                    # Distributed tracing
│   └── logging.py                    # Structured logging
└── storage/                          # 💾 Abstracted storage layer (good pattern!)
    ├── audit.py                      # Audit log persistence
    └── cache.py                      # Redis caching layer

```

**Migration Opportunity:** Consider adopting `rcf/` module structure and observability patterns from estimator-project into efofx-estimate.

---

## Integration Points

### FastAPI Backend → MCP Functions

```
efofx-estimate (Python/FastAPI)
    ↓ HTTP calls
estimator-mcp-functions (Node.js/Serverless)
    ↓ MongoDB queries
MongoDB Atlas (shared database)
```

**Call Pattern:**
1. User submits estimation request → FastAPI
2. FastAPI calls MCP `/reference_classes-query` to find best match
3. FastAPI calls MCP `/adjustments-apply` to calculate regional factors
4. FastAPI calls OpenAI for narrative generation
5. FastAPI stores result in MongoDB
6. FastAPI returns response to user

### Shared Resources

- **MongoDB Database:** Both parts connect to same MongoDB Atlas cluster
- **JWT Tokens:** Both validate same tenant JWT format
- **Reference Class Data:** Both read from same `reference_classes` collection

---

## Entry Points Summary

| Part | Entry Point | Purpose |
|------|-------------|---------|
| efofx-estimate | `app/main.py` | FastAPI server initialization |
| estimator-mcp-functions | `packages/estimator/*/index.js` | Individual serverless functions |
| estimator-project | `app/main.py` | Reference architecture (not actively deployed) |

---

## Development Workflow

**Running efofx-estimate locally:**
```bash
cd apps/efofx-estimate
python -m uvicorn app.main:app --reload
```

**Deploying MCP functions:**
```bash
cd apps/estimator-mcp-functions
doctl serverless deploy .
```

**Testing:**
```bash
# efofx-estimate
cd apps/efofx-estimate
pytest

# MCP functions
cd apps/estimator-mcp-functions
npm test
```
