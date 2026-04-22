# Efofx Project Documentation Index

**Generated:** 2025-11-09
**Project:** efOfX Estimation Service
**Type:** Monorepo with 2 active backend parts
**Documentation Level:** Deep Scan

---

## Project Overview

### Quick Reference

**Repository Type:** Monorepo
**Primary Language:** Python 3.9+ (efofx-estimate), Node.js 18+ (MCP functions)
**Architecture Pattern:** Microservices (FastAPI backend + Serverless functions)
**Database:** MongoDB Atlas (shared)
**AI/LLM:** OpenAI GPT-4

### Parts

#### efofx-estimate (Primary Backend)
- **Type:** Backend (Python/FastAPI)
- **Tech Stack:** FastAPI 0.116.1, MongoDB (Motor 3.3.2), OpenAI 1.51.0, Pydantic 2.11.7
- **Root:** `apps/efofx-estimate/`
- **Entry Point:** `app/main.py`
- **Purpose:** Core estimation service with AI-powered Reference Class Forecasting

#### estimator-mcp-functions (Serverless API)
- **Type:** Serverless Functions (Node.js)
- **Tech Stack:** Node.js 18+, MongoDB 6.3.0, DigitalOcean Functions
- **Root:** `apps/estimator-mcp-functions/`
- **Purpose:** Lightweight data access functions for reference class queries and adjustments

---

## Generated Documentation

### Architecture & Design

- [Architecture Documentation](./architecture.md) - System architecture, integration patterns, security, and data flow
- [Source Tree Analysis](./source-tree-analysis.md) - Annotated directory structure for both parts

### API Documentation

- [API Contracts - efofx-estimate](./api-contracts-efofx-estimate.md) - RESTful API endpoints for primary backend
- [API Contracts - MCP Functions](./api-contracts-estimator-mcp-functions.md) - Serverless function endpoints

### Data Models

- [Data Models - efofx-estimate](./data-models-efofx-estimate.md) - Pydantic models, MongoDB collections, validation rules

### Development & Deployment

- [Development Guide](./development-guide.md) - Local setup, testing, deployment, and troubleshooting

---

## Existing Documentation

### Primary Backend (efofx-estimate)
- [README.md](../apps/efofx-estimate/README.md) - Project overview and setup
- [app/README.md](../apps/efofx-estimate/app/README.md) - Application structure
- [tests/README.md](../apps/efofx-estimate/tests/README.md) - Testing guide
- [scripts/README.md](../apps/efofx-estimate/scripts/README.md) - Utility scripts

### Serverless Functions (estimator-mcp-functions)
- [README.md](../apps/estimator-mcp-functions/README.md) - Project overview
- [DEPLOYMENT.md](../apps/estimator-mcp-functions/DEPLOYMENT.md) - Deployment instructions

---

## Getting Started

### For AI-Assisted Development

This documentation is optimized for AI agents working on brownfield features. When planning new work:

**For Full-Stack Features:**
1. Review [Source Tree Analysis](./source-tree-analysis.md) for project structure
2. Check [API Contracts - efofx-estimate](./api-contracts-efofx-estimate.md) for existing endpoints
3. Review [Data Models](./data-models-efofx-estimate.md) for data structures
4. Consider integration points between FastAPI and MCP functions

**For Backend-Only Features:**
1. Start with [Data Models](./data-models-efofx-estimate.md) to understand schemas
2. Review [API Contracts](./api-contracts-efofx-estimate.md) for endpoint patterns
3. Check `app/services/` in source tree for business logic patterns

**For Serverless Function Changes:**
1. Review [API Contracts - MCP Functions](./api-contracts-estimator-mcp-functions.md)
2. Check `lib/` directory in source tree for shared utilities

### For Human Developers

**Local Development Setup (efofx-estimate):**
```bash
cd apps/efofx-estimate
python -m venv .venv
source .venv/bin/activate  # or `.venv\Scripts\activate` on Windows
pip install -r requirements.txt
cp .env.example .env  # Configure MongoDB URI, OpenAI key
python -m uvicorn app.main:app --reload
```

**Deploy MCP Functions:**
```bash
cd apps/estimator-mcp-functions
npm install
doctl serverless deploy .
```

**Run Tests:**
```bash
# FastAPI backend
cd apps/efofx-estimate
pytest

# MCP functions
cd apps/estimator-mcp-functions
npm test
```

---

## Project Context

### Current State (Brownfield)

This is an **active brownfield project** with:
- ✅ **Working MongoDB integration** in efofx-estimate
- ✅ **JWT authentication** and multi-tenant support
- ✅ **OpenAI integration** for LLM-powered estimation
- ✅ **Serverless functions** deployed to DigitalOcean
- ✅ **Pydantic models** with validation
- ✅ **Test coverage** (pytest for Python, Vitest for Node.js)

### Reference Architecture

`apps/estimator-project/` contains **newer architectural patterns** to consider for future refactoring:
- Dedicated `rcf/` module for Reference Class Forecasting
- Observability layer with Prometheus metrics
- Abstracted storage layer with Redis caching

See [Source Tree Analysis](./source-tree-analysis.md) for migration opportunities.

---

## Key Technologies

| Technology | Version | Purpose | Part |
|------------|---------|---------|------|
| Python | 3.9+ | Backend runtime | efofx-estimate |
| FastAPI | 0.116.1 | Web framework | efofx-estimate |
| MongoDB | 4.6+ | Database | Both |
| Motor | 3.3.2 | Async MongoDB driver | efofx-estimate |
| Pydantic | 2.11.7 | Data validation | efofx-estimate |
| OpenAI | 1.51.0 | LLM integration | efofx-estimate |
| PyJWT | 2.8.0 | JWT authentication | efofx-estimate |
| Node.js | 18+ | Serverless runtime | MCP functions |
| Zod | 3.22.4 | Data validation | MCP functions |
| Pino | 9.0.0 | Structured logging | MCP functions |

---

## Architecture Highlights

### Multi-Tenant Design

Both parts support multi-tenancy:
- JWT tokens contain `tenant_id` claim
- All database queries scoped by tenant
- Per-tenant OpenAI API keys (encrypted)
- Rate limiting per tenant

### Reference Class Forecasting (RCF)

Core estimation methodology:
1. User describes project via chat
2. System matches to best `ReferenceClass` using keywords/category
3. Baseline estimate calculated from historical `ReferenceProject` data
4. Regional and complexity adjustments applied
5. OpenAI generates narrative explanation
6. Result returned with confidence score and assumptions

### Integration Pattern

```
User Request
    ↓
FastAPI Backend (Python)
    ├→ MongoDB (direct writes)
    ├→ MCP Functions (reference class queries)
    ├→ OpenAI API (narrative generation)
    └→ Response with estimate
```

---

## Next Steps for Development

When using this documentation to plan new features:

1. **Review brainstorming output:** `docs/bmm-brainstorming-session-2025-11-09.md` contains strategic product directions
2. **Run document-project workflow:** You're here! ✅
3. **Create PRD:** Define requirements using brownfield context from this documentation
4. **Design Architecture:** Plan how new features integrate with existing structure
5. **Implement:** Use documented patterns and models as reference

---

## Documentation Maintenance

This documentation was generated by the BMad Method `document-project` workflow.

**To update documentation:**
```bash
# From analyst agent
/bmad:bmm:workflows:document-project
```

**To add deep-dive documentation for specific areas:**
```bash
# Select option 2: Deep-dive into specific area
```

---

_Generated using BMad Method document-project workflow (deep scan)_
