# Codebase Concerns

**Analysis Date:** 2026-02-26

## Tech Debt

**Overly Broad Exception Handling:**
- Issue: 55+ instances of `except Exception` catching all exceptions without proper logging of exception details or type
- Files: `app/services/estimation_service.py`, `app/services/llm_service.py`, `app/api/routes.py`, `app/services/chat_service.py`, `app/services/feedback_service.py`
- Impact: Makes debugging difficult, hides specific error conditions that should be handled differently, fails to distinguish between recoverable and unrecoverable errors
- Fix approach: Replace with specific exception handlers (e.g., `except ValueError`, `except HTTPException`). Add exception type to logging. Create custom exception hierarchy for domain-specific errors.

**Global State in Security Module:**
- Issue: Rate limiter uses in-memory dictionary that grows unbounded and is shared across all requests via global instance
- Files: `app/core/security.py` (lines 152-161)
- Impact: Memory leak - old entries never expire from `rate_limiter.requests` dictionary. Old requests are cleaned per-tenant, but dictionary keys persist indefinitely for new tenants
- Fix approach: Implement TTL-based cleanup, use Redis for rate limiting in production, or add a scheduled cleanup task that purges entries older than the window

**LLM Response Parsing is a Stub:**
- Issue: `_parse_estimation_response()` in `app/services/llm_service.py` (lines 122-145) always returns hardcoded values, ignoring actual LLM response
- Files: `app/services/llm_service.py`, `app/services/estimation_service.py` (lines 266-297)
- Impact: LLM responses are not being parsed or used, defeating purpose of LLM integration. All estimations return same default values regardless of input
- Fix approach: Implement proper structured output parsing from LLM (use OpenAI function calling or JSON mode), validate parsed values, fall back to defaults only on parse failure

**In-Memory Cache Without Proper Cleanup:**
- Issue: `_match_cache` in `app/services/rcf_engine.py` (lines 20-21, 285-289) has no maximum size limit or memory management
- Files: `app/services/rcf_engine.py`
- Impact: Cache can grow unbounded and consume excessive memory over time, especially with many unique project descriptions
- Fix approach: Implement LRU cache with max_items parameter, use `functools.lru_cache` instead of manual dict, or implement size-based eviction

**Database Connection Pool Configuration Missing:**
- Issue: MongoDB connection in `app/db/mongodb.py` (lines 23-37) has no connection pooling parameters
- Files: `app/db/mongodb.py`
- Impact: No control over max pool size, connection timeouts, or retry behavior. Could lead to connection exhaustion under load
- Fix approach: Add `maxPoolSize`, `minPoolSize`, `serverSelectionTimeoutMS` to Motor client configuration

**Synchronous Rate Limiter in Async Context:**
- Issue: Rate limiter uses `datetime.utcnow()` and synchronous operations in async code path
- Files: `app/core/security.py` (lines 128-148)
- Impact: Not thread-safe, could cause race conditions with concurrent requests from same tenant
- Fix approach: Use `datetime.now(timezone.utc)` or async-aware rate limiting solution

## Known Bugs

**LLM Service Fails Silently on Missing API Key:**
- Symptoms: If `OPENAI_API_KEY` is not configured, service initializes but fails on first request with generic "Error generating LLM response"
- Files: `app/services/llm_service.py` (lines 21-22)
- Trigger: Run without `OPENAI_API_KEY` environment variable set
- Workaround: Always set environment variables before startup; currently no validation at initialization

**Missing Database Reference:**
- Issue: `app/core/security.py` line 58 references undefined `DB_COLLECTIONS` but doesn't import it
- Files: `app/core/security.py` (line 58)
- Impact: Will throw `NameError` on any API key validation attempt
- Trigger: Any request using API key authentication
- Fix approach: Add `from app.core.constants import DB_COLLECTIONS` to imports

**Cost Breakdown Rounding Errors:**
- Symptoms: Cost breakdown categories may not sum exactly to P50 cost due to rounding
- Files: `app/services/rcf_engine.py` (lines 334-345)
- Current mitigation: Adjusts largest category to compensate, but can be off by up to $0.01 and logs warning
- Impact: Low (within tolerance), but indicates calculation precision issue
- Recommendation: Use Python `Decimal` for financial calculations instead of float

**Tenant-Specific Reference Class Preference Not Implemented:**
- Issue: Code in `app/services/rcf_engine.py` (lines 222-227) sorts by tenant-specific preference, but no tenant_id filtering in query
- Files: `app/services/rcf_engine.py` (lines 190-202)
- Impact: Will match platform classes even when tenant-specific alternatives exist with same score
- Fix approach: Query should filter `reference_classes` where `tenant_id is None OR tenant_id == tenant_id`

## Security Considerations

**Secrets in Environment Variables Not Validated at Startup:**
- Risk: Application starts even if critical secrets are missing (SECRET_KEY, JWT_SECRET_KEY, ENCRYPTION_KEY)
- Files: `app/core/config.py` (lines 28-32)
- Current mitigation: Pydantic Field(...) makes them required, but only validated when Settings() is instantiated
- Recommendations: Add explicit validation in app startup (`main.py` lifespan), fail fast with clear error message showing which secrets are missing

**JWT Token Creation Uses deprecated algorithm selection:**
- Risk: Hardcoded "HS256" algorithm (line 31 in `app/core/security.py`) instead of config value
- Files: `app/core/security.py` (line 31)
- Current mitigation: Config has JWT_ALGORITHM setting but it's not used
- Recommendations: Use `self.algorithm = settings.JWT_ALGORITHM` instead of hardcoded value

**API Key Passed in Bearer Token Without HTTPS Enforcement:**
- Risk: Lines 77-78 in `app/core/security.py` accept plain API keys in Bearer header regardless of connection security
- Files: `app/core/security.py`, CORS middleware in `app/main.py` (line 57)
- Current mitigation: ALLOWED_ORIGINS defaults to ["*"]
- Recommendations:
  1. Enforce HTTPS in production via middleware or reverse proxy
  2. Restrict ALLOWED_ORIGINS to specific domains
  3. Add warning in logs if ALLOWED_ORIGINS includes "*"
  4. Consider moving to API key headers instead of Bearer scheme

**File Upload Validation Incomplete:**
- Risk: `app/services/estimation_service.py` (lines 108-114) checks content_type and file.size, but file.size may be None
- Files: `app/services/estimation_service.py`
- Impact: Could allow oversized files if size header is missing
- Recommendations: Validate file content before storage, scan for malicious content, use proper file storage (not local filesystem placeholder)

**Rate Limit Not Enforced on All Endpoints:**
- Risk: Only POST endpoints check rate limit (`/api/v1/estimate/start`, `/api/v1/estimate/upload`, `/api/v1/chat/send`, `/api/v1/feedback/submit`)
- Files: `app/api/routes.py`
- Impact: GET endpoints can be called unlimited times to enumerate sessions or exfiltrate data
- Recommendations: Apply rate limiting uniformly or document which endpoints are rate-limited and why

**SQL Injection Not Applicable (Using MongoDB):**
- Current state: Uses Motor async driver with parameterized queries, no risk identified
- Exception: Custom query building should still avoid string interpolation

**CORS Allows All Origins by Default:**
- Risk: Settings in `app/core/config.py` (line 34) and `app/main.py` (line 57) default to ALLOWED_ORIGINS=["*"]
- Files: `app/main.py` (lines 55-61)
- Impact: Any website can call API from browser, enabling credential theft via CSRF
- Recommendations: Set explicit allowed origins in .env, add warning in logs if ["*"] is used, add CSRF token protection

## Performance Bottlenecks

**Keyword Extraction Uses O(n) String Operations:**
- Problem: Line 68 in `app/services/rcf_engine.py` does multiple `.replace()` calls sequentially
- Files: `app/services/rcf_engine.py` (lines 68-71)
- Cause: String operations are fine for small inputs but inefficient for large descriptions
- Improvement path: Use regex compilation once, or use `str.maketrans()` for multiple replacements

**Linear Search Through All Reference Classes:**
- Problem: Every match request iterates through ALL reference classes in database (line 196-197 in `rcf_engine.py`)
- Files: `app/services/rcf_engine.py` (lines 196-197)
- Current: No pagination or early stopping
- Cause: MongoDB indexes exist on `category` and `regions` but query doesn't use them effectively
- Improvement path: Use indexed filter `{"category": category}` (already done), consider pre-filtering by region in query, implement scoring only on candidate set not full collection

**Cache Hit Requires Exact Match on All Parameters:**
- Problem: Cache key includes full description string - cache only hits if exact same project description submitted twice
- Files: `app/services/rcf_engine.py` (lines 24-26)
- Impact: Cache effectiveness is very low with free-text descriptions
- Improvement path: Normalize descriptions before caching (remove extra whitespace, lowercase), or use semantic similarity for cache matching

**No Request-Level Timeouts:**
- Problem: LLM requests (OpenAI API) have no timeout configured
- Files: `app/services/llm_service.py` (lines 37-42)
- Impact: Slow/hung LLM requests can hold resources indefinitely
- Improvement path: Add timeout parameter to client initialization and per-request, implement circuit breaker for failing LLM endpoint

**Estimation Service Creates New Service Instances Per Request:**
- Problem: Dependency injection creates new EstimationService, ChatService, FeedbackService for each request
- Files: `app/api/routes.py` (lines 28-38)
- Impact: Re-initializes LLMService and other dependencies repeatedly instead of reusing singletons
- Improvement path: Use singleton pattern or app-level dependency injection with lifespan context

## Fragile Areas

**Pydantic Model Conversions Between Dict and Object:**
- Files: `app/services/rcf_engine.py` (lines 254-258), `app/services/estimation_service.py` (lines 52, 82, 168)
- Why fragile: Converting MongoDB documents to dicts to Pydantic models to dicts again loses type safety and is error-prone
- Safe modification: Use Motor cursor with type hints, create consistent conversion functions, consider using Beanie ODM
- Test coverage: Limited - ReferenceClass model has tests but conversion path not tested

**LLM Response Parsing Logic:**
- Files: `app/services/llm_service.py` (lines 122-145), `app/services/estimation_service.py` (lines 266-297)
- Why fragile: Response parsing is completely stubbed with hardcoded values. Real LLM responses will never be parsed correctly
- Safe modification: Implement incremental parsing with validation at each step, use try/except for each field, test with actual OpenAI responses
- Test coverage: No tests for response parsing

**Rate Limiter State Management:**
- Files: `app/core/security.py` (lines 125-152)
- Why fragile: Global mutable state, no synchronization for concurrent access, memory leaks on tenant churn
- Safe modification: Replace with Redis-backed rate limiter or use thread-safe data structure with proper locking
- Test coverage: No tests for rate limiter under concurrent load

**Authentication Flow:**
- Files: `app/core/security.py` (lines 56-99, 106-108)
- Why fragile: Missing DB_COLLECTIONS import, hardcoded algorithm, no validation of token claims
- Safe modification: Add proper imports, use settings for algorithm, validate exp/iat claims
- Test coverage: No API authentication tests

**Estimation Session Expiration:**
- Files: `app/services/estimation_service.py` (lines 84-90)
- Why fragile: Session status updated in memory but consistency between in-flight requests and DB not guaranteed
- Safe modification: Use MongoDB atomic transactions for status updates, add tests for concurrent access
- Test coverage: No concurrency tests

## Scaling Limits

**In-Memory Caches Cannot Scale Beyond Single Instance:**
- Current capacity: Limited only by available RAM (unbounded growth)
- Limit: First instance to consume available memory loses cache. Clustered deployments have per-instance caches (no sharing)
- Scaling path: Move to Redis for shared caching across multiple instances, implement cache eviction policies

**Rate Limiter Dictionary Grows With Each Unique Tenant:**
- Current capacity: Theoretically unlimited number of tenants
- Limit: Dictionary key count equals total unique tenants ever requested (no cleanup). Memory per tenant ~200 bytes
- Scaling path: Implement TTL-based cleanup, use Redis with automatic expiration, or implement sliding window algorithm without per-tenant storage

**Database Connection Pool Size Fixed:**
- Current capacity: Motor default (50 connections)
- Limit: More concurrent requests than pool size will wait or timeout
- Scaling path: Make pool size configurable via settings, monitor connection utilization in production

**MongoDB Query on Full Reference Class Collection:**
- Current capacity: Works fine up to ~10,000 reference classes
- Limit: Linear scan through all reference classes becomes slow as collection grows, confidence scoring O(n)
- Scaling path: Implement pre-filtering by region in MongoDB query, add category index (already exists), consider approximate nearest neighbor search

## Dependencies at Risk

**OpenAI API Dependency:**
- Risk: Tight coupling to OpenAI API. If service is unavailable or API key invalid, estimations fail completely
- Impact: LLM classification and response parsing entirely depends on OpenAI
- Migration plan:
  1. Implement abstract LLMProvider interface
  2. Create OpenAI, Mock, and fallback implementations
  3. Allow switching providers via config
  4. Cache LLM responses to reduce API calls

**Motor (Async MongoDB Driver) Pin:**
- Risk: `motor==3.3.2` has no security updates guaranteed after newer versions released
- Impact: Potential security vulnerabilities in async driver
- Migration plan: Regularly update Motor version, monitor releases, test with latest version quarterly

**Deprecated JWT Library:**
- Risk: `python-jose` library is less actively maintained than PyJWT
- Impact: Security vulnerabilities may not be patched quickly
- Migration plan: Migrate to PyJWT for token creation (already imported), remove python-jose dependency

**Pydantic 2.x Migration:**
- Risk: Using recent Pydantic 2.11.7 without full validation patterns (e.g., model_validator)
- Impact: Missing advanced validation capabilities
- Migration plan: No urgent need, but implement model validators for complex business logic validation

## Missing Critical Features

**Logging is Minimal:**
- Problem: Structured logging not implemented. No request ID tracing across services
- Blocks: Can't debug multi-step estimations or trace requests through chat/feedback flow
- Impact: Production issues difficult to diagnose

**Error Recovery Strategy Missing:**
- Problem: No retry logic for failed LLM requests, database operations, or API calls
- Blocks: Transient failures cause estimation to fail completely
- Impact: Poor user experience on temporary network issues

**Audit Trail Not Implemented:**
- Problem: No logging of who accessed what estimations, no change history
- Blocks: Can't track data access for compliance or debug unauthorized access
- Impact: Security audit requirements not met

**Notification System Absent:**
- Problem: No webhooks for estimation completion, no email notifications
- Blocks: Users must poll for results
- Impact: Poor integration with external systems

## Test Coverage Gaps

**Untested Area: API Route Handlers:**
- What's not tested: `/api/v1/estimate/start`, `/api/v1/estimate/{session_id}`, `/api/v1/chat/send`, `/api/v1/feedback/submit`, admin endpoints
- Files: `app/api/routes.py`
- Risk: Route-level error handling not tested, HTTP status codes may be wrong, input validation not verified
- Priority: High - These are external API surface

**Untested Area: LLM Service:**
- What's not tested: `generate_response()`, `classify_project()`, `generate_estimation()`, response parsing logic
- Files: `app/services/llm_service.py`
- Risk: LLM integration completely untested. Fallback behavior not tested. API key validation not tested
- Priority: High - Core service logic

**Untested Area: Estimation Service:**
- What's not tested: `start_estimation()`, `get_estimation()`, `upload_image()`, status transitions, session expiration
- Files: `app/services/estimation_service.py`
- Risk: Session lifecycle not verified, concurrent access not tested, database failures not handled
- Priority: High - Business logic

**Untested Area: Authentication/Authorization:**
- What's not tested: API key validation, JWT token verification, rate limiting, CORS
- Files: `app/core/security.py`
- Risk: Auth bypass possible, rate limiting ineffective, CORS misconfiguration not caught
- Priority: Critical - Security

**Untested Area: Database Connection Management:**
- What's not tested: Connection pooling, health checks, index creation, migration scripts
- Files: `app/db/mongodb.py`
- Risk: Database operations may fail silently, indexes may not exist causing poor performance
- Priority: Medium - Infrastructure

**Untested Area: Reference Service and Chat Service:**
- What's not tested: Most service methods have no unit tests
- Files: `app/services/reference_service.py`, `app/services/chat_service.py`, `app/services/feedback_service.py`, `app/services/tenant_service.py`
- Risk: Service logic untested, error paths not verified
- Priority: Medium - Service layer

**Untested Area: Data Models and Validation:**
- What's not tested: EstimationSession creation, timestamp handling, model serialization, validation rules
- Files: `app/models/estimation.py`, `app/models/chat.py`, `app/models/feedback.py`
- Risk: Invalid data can be created, model constraints not enforced
- Priority: Medium - Data integrity

**Test Coverage Metric:**
- Estimated coverage: ~25% (only RCF engine tested, ~1,300 LOC test code vs ~5,900 LOC app code)
- Test count: 2 test files, limited test methods visible
- Assertion: `tests/services/test_rcf_engine.py` has 871 LOC, `tests/test_reference_class_model.py` has 296 LOC
- Gap: 55% of services untested (6 services total, ~1 tested)

---

*Concerns audit: 2026-02-26*
