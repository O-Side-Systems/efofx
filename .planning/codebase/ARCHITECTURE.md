# Architecture

**Analysis Date:** 2026-02-26

## Pattern Overview

**Overall:** Layered service-oriented architecture with a FastAPI backend and React widget frontend, using MongoDB for persistence and OpenAI for LLM capabilities.

**Key Characteristics:**
- Request-response API layer with dependency injection
- Domain-driven service layer with business logic separation
- MongoDB async driver (Motor) for non-blocking database access
- Multi-tenant support with API key-based authentication
- Modular service composition with lazy initialization
- Shadow DOM isolation for embeddable widget

## Layers

**API Layer (Presentation):**
- Purpose: HTTP endpoint handling, request/response validation, dependency injection
- Location: `apps/efofx-estimate/app/api/routes.py`
- Contains: FastAPI route handlers with Pydantic models for request/response serialization
- Depends on: Service layer, security/auth middleware, core constants
- Used by: HTTP clients (frontend widget, external integrations)

**Service Layer (Business Logic):**
- Purpose: Core estimation, chat, feedback, and tenant management logic
- Location: `apps/efofx-estimate/app/services/`
- Contains: EstimationService, ChatService, FeedbackService, LLMService, ReferenceService, TenantService, RCFEngine
- Depends on: Models, database collections, configuration, external APIs (OpenAI)
- Used by: API layer routes

**Model Layer (Data):**
- Purpose: Pydantic models for data validation and serialization
- Location: `apps/efofx-estimate/app/models/`
- Contains: EstimationRequest/Response, ChatRequest/Response, Tenant, EstimationSession, EstimationResult, CostBreakdown
- Depends on: Pydantic, BSON ObjectId
- Used by: API layer, service layer, database layer

**Database Layer (Persistence):**
- Purpose: MongoDB connection management and collection access
- Location: `apps/efofx-estimate/app/db/mongodb.py`
- Contains: Async MongoDB client initialization, collection getters, connection lifecycle management
- Depends on: Motor (AsyncIO MongoDB driver), configuration
- Used by: Services and models requiring data persistence

**Configuration & Core:**
- Purpose: Application settings, constants, security, validation
- Location: `apps/efofx-estimate/app/core/`
- Contains: Config (BaseSettings), constants (enums, messages, collections), security (JWT, authentication)
- Depends on: Pydantic, environment variables
- Used by: All layers

**Middleware Layer:**
- Purpose: Cross-cutting concerns for HTTP requests
- Location: `apps/efofx-estimate/app/middleware/`
- Contains: Request timing, CORS, trusted host middleware
- Used by: FastAPI application entry point

**Utilities:**
- Purpose: Shared helper functions for calculations, validation, file operations
- Location: `apps/efofx-estimate/app/utils/`
- Contains: `calculation_utils.py`, `validation_utils.py`, `file_utils.py`
- Used by: Services and models

**Frontend (Widget):**
- Purpose: Embeddable estimation interface
- Location: `apps/efofx-widget/src/`
- Contains: React components, TypeScript models, API client, Shadow DOM wrapper
- Depends on: React, TypeScript
- Used by: External web pages via embeddable script

## Data Flow

**Estimation Request Flow:**

1. Client calls `/api/v1/estimate/start` with EstimationRequest (description, region, reference_class)
2. API route extracts tenant from JWT token and checks rate limits
3. EstimationService creates EstimationSession with unique session_id
4. Session saved to MongoDB estimates collection
5. RCFEngine or LLMService processes project description:
   - Classify into reference class
   - Fetch reference projects from database
   - Generate cost breakdown
6. EstimationResult created and returned in EstimationResponse
7. Client receives session_id to poll `/api/v1/estimate/{session_id}` for updates

**Chat Flow:**

1. Client sends ChatRequest with message and session_id to `/api/v1/chat/send`
2. ChatService retrieves or creates ChatSession
3. LLMService generates response based on chat context
4. ChatMessage objects created for user and assistant
5. Context updated with conversation state
6. ChatResponse returned to client
7. Messages persisted to chat_sessions collection

**Feedback Collection:**

1. Client submits FeedbackCreate to `/api/v1/feedback/submit`
2. FeedbackService stores feedback with estimation_session_id reference
3. Feedback indexed for summary queries
4. Aggregation queries answer `/api/v1/feedback/summary` requests

**State Management:**

- Session state: Stored in MongoDB EstimationSession document, updated on each operation
- Chat context: Maintained in ChatSession.context dict for LLM awareness
- Tenant context: Extracted from JWT token on each request (stateless)
- Configuration: Loaded from environment at application startup

## Key Abstractions

**EstimationSession:**
- Purpose: Encapsulates single estimation workflow with chat history and results
- Examples: `apps/efofx-estimate/app/models/estimation.py`
- Pattern: Document model with status tracking, timestamps, and hierarchical data (result contains cost breakdown)

**Service Classes (EstimationService, ChatService, etc):**
- Purpose: Encapsulate business logic for specific domains
- Examples: `apps/efofx-estimate/app/services/estimation_service.py`, `chat_service.py`
- Pattern: Lazy initialization with dependencies, async methods for I/O operations

**RCF Engine (Reference Class Forecasting):**
- Purpose: Core algorithm for cost estimation based on reference projects
- Examples: `apps/efofx-estimate/app/services/rcf_engine.py`
- Pattern: Stateless calculation service with pluggable reference data sources

**LLMService:**
- Purpose: Abstract OpenAI API calls for project classification and response generation
- Examples: `apps/efofx-estimate/app/services/llm_service.py`
- Pattern: Facade pattern encapsulating chat completion calls with prompt engineering

**Pydantic Models as Contracts:**
- Purpose: Define request/response contracts and data validation rules
- Examples: EstimationRequest, EstimationResponse, CostBreakdown
- Pattern: BaseModel subclasses with Field validators, Config.schema_extra for documentation

**Collection Accessors:**
- Purpose: Factory functions for MongoDB collection access
- Examples: `get_estimates_collection()`, `get_chat_sessions_collection()`
- Pattern: Lazy initialization delegating to `get_collection(name)` function

## Entry Points

**Backend API Server:**
- Location: `apps/efofx-estimate/app/main.py`
- Triggers: `python -m uvicorn app.main:app` or `python app/main.py`
- Responsibilities: FastAPI app initialization, middleware setup, router inclusion, database connection lifecycle (lifespan context manager), health check endpoints

**Widget Library:**
- Location: `apps/efofx-widget/src/main.tsx`
- Triggers: Script inclusion in external web page, call to `efofxWidget.init({ containerId: 'widget-container' })`
- Responsibilities: Create React root, render App in Shadow DOM, expose destroy function for cleanup

**Tests:**
- Location: `apps/efofx-estimate/tests/`
- Triggers: `pytest`
- Responsibilities: Test fixtures for async client, sample data, database connection, service integration tests

## Error Handling

**Strategy:** Synchronous exception handling with HTTP status code mapping

**Patterns:**

- Service methods raise exceptions on failure; API routes catch and convert to HTTPException
- HTTPException includes status_code from HTTP_STATUS constants and detail message from API_MESSAGES constants
- Logging at error level in try-except blocks with context (tenant_id, session_id, user input)
- Validation errors from Pydantic models automatically return 422 Unprocessable Entity
- Database errors mapped to 500 Internal Server Error with generic message to client
- Rate limiting errors return 429 status via check_rate_limit dependency
- Missing resources return 404 with specific message (e.g., "Estimation session not found")

Example: `apps/efofx-estimate/app/api/routes.py` lines 43-58 (start_estimation endpoint)

## Cross-Cutting Concerns

**Logging:**
- Framework: Standard Python logging module
- Pattern: Logger created per module with `__name__`
- Where used: Every exception, entry/exit points, significant state changes
- Format: Contextual strings with f-strings, error tracebacks on exceptions

**Validation:**
- Input: Pydantic models enforce schema, type hints, and custom validators at API boundary
- Business: Service methods validate state transitions (e.g., session status checks)
- File uploads: Check mime type and size in FileUploadConfig constants

**Authentication:**
- Method: JWT tokens in Authorization header
- Implementation: `get_current_tenant` dependency extracts and validates token
- Tenant isolation: Services filter by tenant_id, preventing cross-tenant data access
- Example: `apps/efofx-estimate/app/core/security.py`

**Rate Limiting:**
- Method: Per-tenant rate limit check via `check_rate_limit(tenant_id)` function
- Config: RATE_LIMIT_ENABLED, RATE_LIMIT_PER_MINUTE from settings
- Example: Called in API routes before resource-intensive operations (lines 50, 88, 108)

**Multi-Tenancy:**
- Model: Each EstimationSession, ChatSession, Feedback record tagged with tenant_id
- Enforcement: Service methods filter all queries by tenant_id
- Configuration: Tenant can have region restrictions, monthly quotas, custom OpenAI key
- Example: Tenant model in `apps/efofx-estimate/app/models/tenant.py`

