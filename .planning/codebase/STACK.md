# Technology Stack

**Analysis Date:** 2026-02-26

## Languages

**Primary:**
- Python 3.9-3.13 - Backend services and estimation engine
- TypeScript ~5.9.3 - Frontend widget, MCP functions, type safety
- JavaScript (ES modules) - Serverless MCP functions, runtime

**Secondary:**
- CSS/PostCSS - Styling with Tailwind

## Runtime

**Environment:**
- Python 3.9+ (minimum requirement across services)
- Node.js 18+ (for MCP functions and widget)
- ASGI server (uvicorn) for async Python backend

**Package Manager:**
- Python: pip with pyproject.toml and requirements.txt
  - Lockfile: requirements.txt (pinned versions)
- Node.js: npm
  - Lockfile: package-lock.json (managed by npm)

## Frameworks

**Core:**
- FastAPI 0.116.1 - Async REST API framework (`apps/efofx-estimate/app/main.py`, `apps/estimator-project/`)
- React 19.2.0 - Frontend widget UI (`apps/efofx-widget/src/App.tsx`)
- Express/Serverless Functions - Handled via DigitalOcean Functions for MCP endpoints

**Testing:**
- pytest 8.4.1 - Python unit/integration testing (`apps/efofx-estimate/tests/`, `apps/estimator-project/tests/`)
  - pytest-asyncio 0.23.5 - Async test support
  - pytest-cov 6.0.0 - Coverage reporting
- vitest 1.0.0 - TypeScript unit testing for MCP functions (`apps/estimator-mcp-functions/`)
- httpx 0.27.0 - Async HTTP client for testing

**Build/Dev:**
- Vite 7.2.2 - Frontend bundler and dev server (`apps/efofx-widget/`)
- TypeScript 5.9.3 - Type checking
- ESLint 9.39.1 - Linting
- Prettier 3.1.0 - Code formatting
- black 24.1.1 - Python code formatter
- flake8 7.0.0 - Python linter
- mypy 1.8.0 - Python static type checker
- Ruff 0.6.5 - Fast Python linter (in estimator-project)
- gunicorn 21.0.0+ - Production WSGI server for Python apps
- pre-commit 3.6.2 - Git hook framework for code quality checks

## Key Dependencies

**Critical:**
- openai 1.51.0 - OpenAI API client for LLM integration (`apps/efofx-estimate/app/services/llm_service.py`)
- motor 3.3.2 - Async MongoDB driver for Python (`apps/efofx-estimate/app/db/mongodb.py`)
- pymongo 4.6.1 - MongoDB Python driver (used with motor)
- mongodb 6.3.0 - MongoDB Node.js client (`apps/estimator-mcp-functions/lib/db.js`)

**Authentication & Security:**
- python-jose[cryptography] 3.3.0 - JWT implementation for Python
- PyJWT 2.8.0 - JSON Web Token library
- passlib[bcrypt] 1.7.4 - Password hashing
- jsonwebtoken 9.0.2 - JWT for Node.js (`apps/estimator-mcp-functions/lib/auth.js`)
- cryptography >= 41.0.0 - Encryption (Fernet) for BYOK support

**Data Validation:**
- pydantic 2.11.7 - Data validation and serialization
- pydantic-settings 2.2.1 - Environment variable handling
- zod 3.22.4 - Schema validation for Node.js

**Infrastructure:**
- redis 5.0.8 - Redis client (optional, configured in estimator-project)
- prometheus-client 0.22.1 - Prometheus metrics
- structlog >= 24.0.0 - Structured logging

**Utilities:**
- python-dotenv 1.0.1 - Environment file loading
- python-multipart 0.0.9 - Multipart form handling
- python-dateutil 2.8.2 - Date/time utilities
- lru-cache 10.1.0 - In-memory caching for Node.js
- pino 9.0.0 - Logger for Node.js

## Configuration

**Environment:**
- Settings loaded from `.env` files via pydantic BaseSettings
- Environment template available at `scripts/.env.template`
- Critical config in `apps/efofx-estimate/app/core/config.py`
- Secondary config in `apps/estimator-project/app/core/config.py`

**Key Configuration Files:**
- `pyproject.toml` - Python project metadata and build configuration
- `package.json` - Node.js project dependencies and scripts
- `.env` files - Environment variables (never committed)
- `tsconfig.json` - TypeScript compiler options (efofx-widget)
- ESLint/Prettier configs - Code quality rules

**Build Configuration:**
- DigitalOcean App Platform: `.do/app.yaml` at `apps/efofx-estimate/.do/app.yaml`
  - Auto-deployment on push to main
  - Python environment with gunicorn/uvicorn workers
- Vite config for frontend: `apps/efofx-widget/` (default config)
- MCP Functions deployment: `project.yml` at `apps/estimator-mcp-functions/project.yml`

## Platform Requirements

**Development:**
- Python 3.9+ with pip
- Node.js 18+ with npm
- PostgreSQL/MongoDB client tools (optional)
- Git for version control
- Pre-commit hooks for code quality

**Production:**
- Deployment Target: DigitalOcean App Platform
  - Region: nyc (New York)
  - Instance Size: basic-xs (0.5 vCPU, 512 MB RAM)
  - Instance Count: 1 (auto-scalable)
- Database: MongoDB Atlas (cloud-hosted)
- LLM Provider: OpenAI API (cloud-based)
- Optional: Redis for caching/session management
- Optional: Prometheus for metrics collection

**Containerization:**
- No explicit Docker configuration found in main codebase
- DigitalOcean App Platform handles containerization automatically

## Database

**Primary:**
- MongoDB (Atlas) - Main data store
  - Client: motor (async Python), mongodb (Node.js)
  - Collections managed in `apps/efofx-estimate/app/db/mongodb.py`

**Optional:**
- Redis - Cache/session store (configured but optional in estimator-project)
- Audit database - PostgreSQL optional for audit logs (configured in estimator-project)

## Observability

**Logging:**
- structlog 24.1.0 - Structured logging for Python
- pino 9.0.0 - Structured logging for Node.js
- Log format: JSON (configurable)
- Sentry integration removed (flags in config for future re-enablement)

**Metrics:**
- Prometheus client for metrics collection
- Prometheus port: 9000 (from estimator-project config)

---

*Stack analysis: 2026-02-26*
