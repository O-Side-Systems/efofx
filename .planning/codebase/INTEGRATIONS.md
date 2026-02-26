# External Integrations

**Analysis Date:** 2026-02-26

## APIs & External Services

**Language Model/AI:**
- OpenAI (GPT-4) - LLM for project classification and estimation generation
  - SDK/Client: openai 1.51.0 (Python), openai 1.101.0 (Node.js in estimator-project)
  - Auth: OPENAI_API_KEY (environment variable, type: SECRET)
  - Usage: `apps/efofx-estimate/app/services/llm_service.py`
    - generate_response() - Text generation with system prompts
    - classify_project() - Project classification into reference classes
    - generate_estimation() - Structured estimate generation with RCF data
  - Configuration:
    - Model: gpt-4 or gpt-4-turbo-preview
    - Max tokens: 4000
    - Temperature: 0.7

## Data Storage

**Databases:**
- MongoDB Atlas (cloud-hosted)
  - Connection: MONGO_URI (environment variable, type: SECRET)
  - Client: motor 3.3.2 (Python async), mongodb 6.3.0 (Node.js)
  - Database name: efofx_estimate (dev) or efofx_production (production)
  - Collections defined in `apps/efofx-estimate/app/core/constants.py`:
    - TENANTS - Tenant/account management
    - REFERENCE_CLASSES - RCF classification data
    - REFERENCE_PROJECTS - Historical project data for RCF
    - ESTIMATES - Generated estimates and results
    - FEEDBACK - User feedback on estimates
    - CHAT_SESSIONS - Chat conversation history
  - Indexes: Automatically created at startup by `apps/efofx-estimate/app/db/mongodb.py:create_indexes()`
  - Connection pool: max 10 connections (MCP functions), configurable in efofx-estimate

**File Storage:**
- Local filesystem only
  - No cloud storage integration (S3, GCS, etc.)
  - Image upload support: JPEG, PNG, WebP (max 10MB)

**Caching:**
- Redis (optional, configurable)
  - Connection: REDIS_URL (environment variable, optional)
  - Used in: estimator-project for session/cache management
  - Package: redis 5.0.8
  - TTL configuration: CACHE_TTL (default 120 seconds)
  - Not yet integrated in main efofx-estimate service

## Authentication & Identity

**Auth Provider:**
- Custom JWT-based implementation
  - Token type: JSON Web Tokens (RS256 or HS256)
  - Libraries:
    - Python: python-jose[cryptography] 3.3.0, PyJWT 2.8.0
    - Node.js: jsonwebtoken 9.0.2
  - Implementation:
    - `apps/estimator-mcp-functions/lib/auth.js`:
      - verifyJmac() - HMAC-SHA256 signature verification with timestamp/nonce
      - verifyJwt() - RS256 JWT verification
      - extractTenantId() - Multi-source tenant ID extraction
    - `apps/efofx-estimate/app/core/security.py` - Token generation and validation

**Key Management:**
- JWT Configuration:
  - Algorithm: HS256 or RS256
  - Expiration: 24 hours (configurable)
  - Public key: JWT_PUBLIC_KEY_PEM (RS256 for MCP)
  - Private key: MCP_JWT_PRIVATE_KEY (for token signing)
  - Issuer: efofx-monolith or efofx-estimate
  - Audience: efofx-mcp

**HMAC Authentication:**
- For MCP function calls:
  - Key ID: HMAC_KEY_ID (x-efofx-key-id header)
  - Secret: HMAC_SECRET_B64 (base64-encoded)
  - Signature: x-efofx-signature (HMAC-SHA256)
  - Timestamp: x-efofx-timestamp (Unix seconds, ±120s tolerance)
  - Nonce: x-efofx-nonce (replay protection)

**Tenant Isolation:**
- Multi-tenant support via tenant_id parameter
- Stored in MongoDB tenants collection
- API key per tenant for authentication

## Monitoring & Observability

**Error Tracking:**
- Sentry integration removed (configuration preserved for future use)
- In-app error handling with logging

**Logs:**
- Structured JSON logging
  - Python: structlog 24.1.0
  - Node.js: pino 9.0.0
  - Log level: INFO (production), configurable via LOG_LEVEL env var
  - Format: JSON (standard format across services)

**Metrics:**
- Prometheus client 0.22.1 for metrics collection
- Prometheus port: 9000
- Metrics available at standard Prometheus endpoint

**Health Checks:**
- HTTP endpoint: GET /health
- Checks:
  - Service status: "healthy" or "degraded"
  - Database connectivity
  - Version information
- DigitalOcean App Platform health check:
  - Path: /health
  - Initial delay: 10 seconds
  - Period: 10 seconds
  - Timeout: 5 seconds
  - Failure threshold: 3

## CI/CD & Deployment

**Hosting:**
- DigitalOcean App Platform (primary)
  - Service: efofx-api
  - Region: nyc (New York)
  - Instance: basic-xs (0.5 vCPU, 512 MB)
  - Auto-deployment on push to main branch
  - Build command: pip install -r requirements.txt
  - Run command: gunicorn -w 2 -k uvicorn.workers.UvicornWorker app.main:app
  - Port: 8080

**CI Pipeline:**
- GitHub integration for auto-deployment
  - Repository: brettlee/efofx-workspace
  - Branch: main (auto-deploy enabled)
  - No explicit GitHub Actions workflow (DigitalOcean handles builds)

**Serverless Functions:**
- DigitalOcean Functions for MCP endpoints
  - Deployment: doctl serverless deploy
  - Runtime: Node.js 18+
  - Package: estimator-mcp-functions
  - Manifest: project.yml in root

## Environment Configuration

**Required Environment Variables (Production):**

*Security:*
- SECRET_KEY - Application secret for session signing
- JWT_SECRET_KEY - JWT token signing secret
- ENCRYPTION_KEY - Fernet encryption for sensitive data
- HMAC_KEY_ID - HMAC key identifier
- HMAC_SECRET_B64 - Base64-encoded HMAC secret
- JWT_PUBLIC_KEY_PEM - RS256 public key (PEM format)
- MCP_JWT_PRIVATE_KEY - RSA private key for MCP token signing

*Database:*
- MONGO_URI - MongoDB Atlas connection string (mongodb+srv://...)
- MONGO_DB_NAME - Database name (default: efofx_estimate)

*LLM:*
- OPENAI_API_KEY - OpenAI API key (sk-...)
- OPENAI_MODEL - Model identifier (gpt-4, gpt-4-turbo-preview)

*Infrastructure:*
- MCP_BASE_URL - Base URL for MCP functions
- MCP_HMAC_KEY_ID - Key ID for MCP HMAC verification
- MCP_HMAC_SECRET - Secret for MCP HMAC

*Optional:*
- REDIS_URL - Redis connection string (optional caching)
- AUDIT_DB_URI - PostgreSQL connection for audit logs
- SENTRY_DSN - Sentry error tracking (disabled by default)

**Configuration Sources:**
- Environment files: .env (development), DigitalOcean secrets (production)
- Secret management: DigitalOcean App Platform secrets
- No hardcoded secrets in codebase

## Webhooks & Callbacks

**Incoming:**
- No external webhooks currently implemented
- MCP functions receive HTTP POST requests for estimation operations

**Outgoing:**
- None implemented
- Future consideration: Webhook delivery for estimation completion events

## CORS & Network Configuration

**Allowed Origins (Production):**
- https://widget.efofx.ai
- https://app.efofx.ai
- Configurable via ALLOWED_ORIGINS env var (default: "*" in dev)

**Allowed Methods:**
- GET, POST, PUT, DELETE, OPTIONS, PATCH

**CORS Middleware:**
- Implemented in `apps/efofx-estimate/app/main.py`
- FastAPI CORSMiddleware with configurable origins

**Rate Limiting:**
- Enabled by default (RATE_LIMIT_ENABLED: true)
- Rate: 60 requests per minute
- Implementation: slowapi or custom middleware
- Configurable via RATE_LIMIT_PER_MINUTE env var

## MCP Function Integration Points

**Reference Classes Operations:**
- Endpoint: GET /reference_classes-query
- Function: `apps/estimator-mcp-functions/packages/estimator/reference_classes-query/`
- Returns: List of reference classes for given region/category

**Reference Class Details:**
- Endpoint: GET /reference_classes-get
- Function: `apps/estimator-mcp-functions/packages/estimator/reference_classes-get/`
- Returns: Detailed data for specific reference class

**Adjustments:**
- Endpoint: POST /adjustments-apply
- Function: `apps/estimator-mcp-functions/packages/estimator/adjustments-apply/`
- Input: Base estimate + adjustment factors (region, complexity, etc.)
- Output: Adjusted estimate with breakdown

**Manifest:**
- Endpoint: GET /manifest
- Function: `apps/estimator-mcp-functions/packages/estimator/manifest/`
- Returns: Available operations and schemas

---

*Integration audit: 2026-02-26*
