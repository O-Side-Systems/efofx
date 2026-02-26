# Codebase Structure

**Analysis Date:** 2026-02-26

## Directory Layout

```
efofx-workspace/
├── apps/                          # Application packages
│   ├── efofx-estimate/            # Backend estimation API (Python/FastAPI)
│   ├── efofx-widget/              # Frontend widget (TypeScript/React)
│   ├── estimator-mcp-functions/   # MCP function definitions
│   ├── estimator-project/         # Legacy estimator application
│   └── synthetic-data-generator/  # Test data generation utility
├── bmad/                          # Internal agent framework
├── docs/                          # Documentation and specifications
├── .planning/                     # Planning and analysis documents
└── scripts/                       # Deployment and utility scripts
```

## Directory Purposes

**apps/efofx-estimate/ (Backend Core):**
- Purpose: Main FastAPI application serving estimation API, chat, feedback, and reference data
- Contains: Python packages with layered architecture (api, services, models, db)
- Key files: `app/main.py` (entry point), `app/api/routes.py` (endpoints), `tests/` (test suite)

**apps/efofx-estimate/app/ (Application Code):**
- Purpose: Core application source code organized by architectural layer
- Subdirectories:
  - `api/` - HTTP route handlers
  - `services/` - Business logic layer
  - `models/` - Pydantic data models
  - `db/` - Database connection and collection access
  - `core/` - Configuration, constants, security
  - `middleware/` - HTTP middleware
  - `utils/` - Shared utility functions

**apps/efofx-estimate/app/services/:**
- Purpose: Service implementations for specific business domains
- Contains:
  - `estimation_service.py` - Orchestrates estimation workflow
  - `chat_service.py` - Manages chat sessions and conversations
  - `feedback_service.py` - Handles feedback submission and aggregation
  - `llm_service.py` - Wraps OpenAI API calls
  - `reference_service.py` - Manages reference classes and projects
  - `tenant_service.py` - Multi-tenant management
  - `rcf_engine.py` - Reference Class Forecasting algorithm (18+ KB)

**apps/efofx-estimate/app/models/:**
- Purpose: Pydantic BaseModel definitions for type safety and validation
- Contains:
  - `estimation.py` - EstimationRequest, EstimationSession, EstimationResponse, EstimationResult, CostBreakdown
  - `chat.py` - ChatRequest, ChatResponse, ChatMessage, ChatSession
  - `tenant.py` - Tenant, TenantCreate, TenantUpdate, PyObjectId (custom ObjectId)
  - `feedback.py` - FeedbackCreate, FeedbackResponse
  - `reference_class.py` - ReferenceClass, ReferenceProject models
  - `reference.py` - Additional reference data models

**apps/efofx-estimate/app/core/:**
- Purpose: Application-wide configuration and cross-cutting concerns
- Contains:
  - `config.py` - Settings class with environment variable bindings (75 lines)
  - `constants.py` - Enums (EstimationStatus, Region, ReferenceClassCategory), API messages, estimation config, LLM prompts, database collection names, HTTP status codes (147 lines)
  - `security.py` - JWT authentication, rate limiting, tenant extraction

**apps/efofx-estimate/app/db/:**
- Purpose: Data persistence layer with MongoDB access
- Contains:
  - `mongodb.py` - AsyncIOMotorClient setup, connection lifecycle, collection getters
  - Global _client and _database variables for singleton access

**apps/efofx-estimate/app/utils/:**
- Purpose: Shared helper functions used across services
- Contains:
  - `calculation_utils.py` - Math operations for cost/timeline calculations
  - `validation_utils.py` - Input validation helper functions
  - `file_utils.py` - Image upload and file handling

**apps/efofx-estimate/tests/:**
- Purpose: Test suite with pytest fixtures and integration tests
- Contains:
  - `conftest.py` - Session/fixture setup for async testing (154 lines with sample data)
  - `test_reference_class_model.py` - Reference class model tests
  - `services/` - Service layer integration tests
  - `api/` - API endpoint tests
  - `fixtures/` - Reusable test data

**apps/efofx-widget/:**
- Purpose: Embeddable React widget for estimation chat interface
- Contains: TypeScript/React source, build configuration, public assets

**apps/efofx-widget/src/:**
- Purpose: Frontend widget source code
- Contains:
  - `main.tsx` - Widget initialization function exposed as library (50 lines)
  - `App.tsx` - Root component with placeholder UI
  - `App.css` - Styling
  - `components/ShadowDOMWrapper.tsx` - Shadow DOM isolation for embeddability
  - `api/` - API client functions
  - `types/` - TypeScript interfaces
  - `hooks/` - Custom React hooks
  - `assets/` - Images and static files

**docs/:**
- Purpose: Product, architecture, and test documentation
- Contains:
  - `PRD/` - Product requirements
  - `stories/` - User stories and epic descriptions
  - `test-guides/` - QA test procedures and guides

**bmad/:**
- Purpose: Internal agent framework for orchestration and planning
- Contains: Agent definitions, tasks, workflows, and test architecture

**.planning/codebase/:**
- Purpose: Generated analysis documents for codebase navigation
- Contains: ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md (generated by /gsd:map-codebase)

## Key File Locations

**Entry Points:**

- `apps/efofx-estimate/app/main.py`: FastAPI application initialization, lifespan management, health endpoints
- `apps/efofx-widget/src/main.tsx`: Widget library entry point with init() function
- `apps/efofx-estimate/tests/conftest.py`: Test configuration and fixtures

**Configuration:**

- `apps/efofx-estimate/app/core/config.py`: Pydantic BaseSettings with environment variable bindings
- `apps/efofx-estimate/app/core/constants.py`: Enums, messages, database collection names, HTTP codes
- `.env` file (not committed): Runtime configuration with secrets

**Core Logic:**

- `apps/efofx-estimate/app/services/estimation_service.py`: EstimationService with start/get/upload workflows
- `apps/efofx-estimate/app/services/rcf_engine.py`: Reference Class Forecasting algorithm (18.2 KB)
- `apps/efofx-estimate/app/services/chat_service.py`: ChatService for conversational estimation
- `apps/efofx-estimate/app/services/llm_service.py`: LLMService wrapping OpenAI API

**Data Models:**

- `apps/efofx-estimate/app/models/estimation.py`: EstimationRequest/Session/Response/Result (211 lines)
- `apps/efofx-estimate/app/models/tenant.py`: Multi-tenant models with ObjectId support (117 lines)
- `apps/efofx-estimate/app/models/chat.py`: Chat data models

**Database:**

- `apps/efofx-estimate/app/db/mongodb.py`: MongoDB async client and collection management
- Collections: tenants, estimates, reference_classes, reference_projects, feedback, chat_sessions

**API Routes:**

- `apps/efofx-estimate/app/api/routes.py`: All HTTP endpoints (239 lines)
  - POST /api/v1/estimate/start - Start estimation
  - GET /api/v1/estimate/{session_id} - Get estimation status
  - POST /api/v1/estimate/{session_id}/upload - Upload image
  - POST /api/v1/chat/send - Send chat message
  - GET /api/v1/chat/{session_id}/history - Get chat history
  - POST /api/v1/feedback/submit - Submit feedback
  - GET /api/v1/feedback/summary - Get feedback summary
  - GET /api/v1/status - Service status
  - GET /api/v1/admin/* - Admin endpoints

**Testing:**

- `apps/efofx-estimate/tests/conftest.py`: Global pytest fixtures with sample data
- `apps/efofx-estimate/tests/test_reference_class_model.py`: Model validation tests
- `apps/efofx-estimate/tests/services/` - Service integration tests
- `apps/efofx-estimate/tests/api/` - API endpoint tests

## Naming Conventions

**Files:**

- Snake case: `estimation_service.py`, `reference_class.py`
- Test files: `test_*.py` or `*_test.py` (pytest pattern)
- Models: `{domain}.py` e.g., `estimation.py`, `chat.py`, `tenant.py`

**Directories:**

- Lowercase plural for collections: `services/`, `models/`, `utils/`, `tests/`
- Lowercase for packages: `core/`, `db/`, `api/`
- App root package: `app/`

**Python Classes:**

- PascalCase: EstimationService, ChatService, ReferenceClassCategory, EstimationRequest
- Enum names: PascalCase with str Enum base for JSON compatibility
- Pydantic models: PascalCase with Config inner class for schema documentation

**Python Functions:**

- Snake case: `start_estimation()`, `get_estimates_collection()`, `check_rate_limit()`
- Async functions use `async def` with `await` for I/O operations
- Dependency injection functions: `get_*()` (e.g., `get_current_tenant()`)

**API Routes:**

- RESTful path patterns: `/api/v1/{resource}/{id}/{action}`
- HTTP verbs: POST for creation, GET for retrieval, PUT for full update
- Resource plural: `/estimate/`, `/chat/`, `/feedback/`
- Session identifier: `session_id` as path parameter or request body field

## Where to Add New Code

**New API Endpoint:**
1. Add Pydantic request/response models to `apps/efofx-estimate/app/models/{domain}.py`
2. Add route handler to `apps/efofx-estimate/app/api/routes.py`
3. Implement business logic in new or existing service: `apps/efofx-estimate/app/services/{domain}_service.py`
4. Add service dependency injection function in routes.py
5. Add tests in `apps/efofx-estimate/tests/api/` and `tests/services/`

**New Service Layer Feature:**
1. Create new service class in `apps/efofx-estimate/app/services/{feature}_service.py`
2. Inject dependencies (LLMService, ReferenceService, MongoDB collections) in `__init__`
3. Use async/await for I/O operations
4. Add error handling with logging
5. Write tests in `apps/efofx-estimate/tests/services/test_{feature}_service.py`

**New Data Model:**
1. Add Pydantic BaseModel to `apps/efofx-estimate/app/models/{domain}.py`
2. Include Field descriptions and schema_extra examples
3. Add Config class with arbitrary_types_allowed=True if using ObjectId
4. Test validation in `tests/`

**Widget Component:**
1. Add React component to `apps/efofx-widget/src/components/{ComponentName}.tsx`
2. Use TypeScript interfaces from `apps/efofx-widget/src/types/`
3. Use API client from `apps/efofx-widget/src/api/`
4. Style with CSS modules or inline in component

**Utility Function:**
1. Categorize by function type: `calculation_utils.py`, `validation_utils.py`, `file_utils.py`
2. Export from `apps/efofx-estimate/app/utils/__init__.py`
3. Document with docstrings
4. Test in `tests/` with unit tests

**New Test:**
1. Use fixtures from `tests/conftest.py` (sample_tenant, sample_estimation_request, etc.)
2. Use async/await with pytest.mark.asyncio for async functions
3. Clean up database state after tests using fixtures with yield
4. Follow pattern in existing tests for consistency

## Special Directories

**apps/efofx-estimate/.venv/:**
- Purpose: Python virtual environment for isolated dependencies
- Generated: Yes (by `python -m venv .venv`)
- Committed: No (.gitignore excludes)

**apps/efofx-estimate/.pytest_cache/:**
- Purpose: Pytest cache for faster test runs
- Generated: Yes (automatically by pytest)
- Committed: No (.gitignore excludes)

**apps/efofx-estimate/efofx_estimate.egg-info/:**
- Purpose: Package metadata from setup.py installation
- Generated: Yes (by `pip install -e .`)
- Committed: No (.gitignore excludes)

**apps/efofx-widget/node_modules/:**
- Purpose: npm dependencies
- Generated: Yes (by `npm install`)
- Committed: No (.gitignore excludes)

**apps/efofx-widget/dist/:**
- Purpose: Built/bundled widget output
- Generated: Yes (by `npm run build`)
- Committed: No (.gitignore excludes)

**apps/efofx-estimate/.do/:**
- Purpose: DigitalOcean App Platform configuration
- Generated: No (manually maintained)
- Committed: Yes

**apps/synthetic-data-generator/:**
- Purpose: Utility for generating test data and reference projects
- Generated: No (source code)
- Committed: Yes (contains Python generators)

