# efOfX Estimation Service

## What This Is

An AI-powered, multi-tenant SaaS platform that transforms project estimation from guesswork into transparent, data-driven forecasting. Uses Reference Class Forecasting (RCF) methodology combined with LLM-powered narrative generation to help contractors provide accurate project estimates while teaching them to communicate uncertainty effectively. MVP focuses on construction/home improvement estimation with a domain-agnostic backend.

## Core Value

Trust through transparency — probabilistic estimates (P50/P80 ranges) with explainable breakdowns that help contractors educate customers, not just quote them. The self-improving feedback loop is the competitive moat.

## Requirements

### Validated

- ✓ Vite + React 19 + TypeScript widget project scaffolded with Shadow DOM — Epic 1
- ✓ FastAPI backend with structured folder layout and core dependencies — Epic 1
- ✓ DigitalOcean App Platform deployment with auto-deploy from GitHub — Epic 1
- ✓ MongoDB Atlas connection with Motor async driver and Sentry monitoring — Epic 1
- ✓ Domain-agnostic reference class MongoDB schema with flexible attributes — Epic 2
- ✓ RCF matching algorithm with keyword scoring and confidence thresholds — Epic 2
- ✓ Synthetic construction reference classes (7 types × 4 regions) — Epic 2
- ✓ Baseline P50/P80 estimate calculation with cost breakdown — Epic 2
- ✓ Regional and complexity adjustment factors applied to estimates — Epic 2

### Active

- [ ] Tenant registration and management with email verification (Epic 3)
- [ ] JWT authentication with tenant_id claims and token management (Epic 3)
- [ ] BYOK encryption for OpenAI API keys using Fernet (Epic 3)
- [ ] Tenant isolation middleware enforcing data boundaries (Epic 3)
- [ ] Per-tenant rate limiting by tier (Epic 3)
- [ ] MongoDB indexes for tenant isolation performance (Epic 3)
- [ ] OpenAI client with BYOK support for LLM calls (Epic 4)
- [ ] Git-based prompt management system (Epic 4)
- [ ] LLM-generated estimate narratives for stakeholder communication (Epic 4)
- [ ] Conversational scoping chat engine for project details (Epic 4)
- [ ] LLM response caching for cost efficiency (Epic 4)
- [ ] Shadow DOM widget container for contractor site embedding (Epic 5)
- [ ] Widget branding configuration system (colors, logos, text) (Epic 5)
- [ ] Conversational chat UI within widget (Epic 5)
- [ ] Lead capture form in widget flow (Epic 5)
- [ ] Estimate results display in widget (Epic 5)
- [ ] Widget security hardening (Epic 5)
- [ ] Widget embed code and documentation (Epic 5)
- [ ] Widget analytics and monitoring (Epic 5)
- [ ] Customer feedback via magic link system (Epic 6)
- [ ] Contractor feedback dashboard (Epic 6)
- [ ] Calibration metrics calculation from actual outcomes (Epic 6)
- [ ] Calibration dashboard for tenants (Epic 6)
- [ ] Synthetic data validation and tuning from real feedback (Epic 6)
- [ ] LLM prompt refinement from feedback patterns (Epic 6)
- [ ] Shared backend utilities extraction (Epic 7)
- [ ] Shared frontend components library (Epic 7)
- [ ] YAGNI pass — remove unused code and features (Epic 7)
- [ ] Code quality standards and documentation (Epic 7)

### Out of Scope

- Communication coaching UI (internal/external narrative toggle) — post-MVP
- Multiple LLM provider support — OpenAI BYOK sufficient for MVP
- Real-time collaboration — async estimation is fine
- Mobile apps — web widget + API sufficient
- Advanced analytics dashboard — basic calibration metrics only
- ML/NLP matching — simple keyword matching for MVP, upgrade later

## Context

- **Project state**: Brownfield MVP — Epics 1-2 complete (foundation + RCF engine)
- **Existing apps**: `apps/efofx-estimate/` (FastAPI backend), `apps/efofx-widget/` (React widget), `apps/estimator-mcp-functions/` (MCP serverless), `apps/synthetic-data-generator/`
- **Deployment**: DigitalOcean App Platform with auto-deploy
- **Database**: MongoDB Atlas with Motor async driver
- **Synthetic data**: ~100 construction reference classes seeded
- **PRD**: Comprehensive PRD at `docs/PRD.md` with 7 epics, 44-51 stories
- **Architecture**: Detailed architecture doc at `docs/architecture.md`
- **Story files**: All stories drafted in `docs/stories/`

## Constraints

- **Tech stack**: Python FastAPI backend, React/TypeScript widget, MongoDB Atlas — established in Epic 1
- **LLM provider**: OpenAI only for MVP (BYOK model)
- **Scale target**: 15 active tenants, 100k estimates/month, 99.5% uptime
- **Security**: Zero cross-tenant data leakage — hard isolation required
- **Deployment**: DigitalOcean App Platform (already configured)
- **Domain**: Construction/home improvement MVP, IT/dev fast follow

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Domain-agnostic backend with flexible attributes | Support multiple verticals without schema changes | ✓ Good — proven in Epic 2 |
| Synthetic data with statistical distributions | Bootstrap estimates before real feedback exists | ✓ Good — validated against HomeAdvisor data |
| Simple keyword matching for RCF | Ship fast, upgrade to ML/NLP later | ✓ Good — sufficient for MVP |
| Shadow DOM for widget isolation | Prevent CSS/JS conflicts on contractor sites | — Pending |
| BYOK for OpenAI keys | Tenants control their own LLM costs | — Pending |
| Fernet encryption for stored API keys | Symmetric encryption sufficient for single-server MVP | — Pending |
| JWT with tenant_id claims | Stateless auth with built-in tenant context | — Pending |

---
*Last updated: 2026-02-26 after initialization (Epics 1-2 complete, starting Epics 3-7)*
