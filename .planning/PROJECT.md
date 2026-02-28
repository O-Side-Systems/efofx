# efOfX Estimation Service

## What This Is

An AI-powered, multi-tenant SaaS platform that transforms project estimation from guesswork into transparent, data-driven forecasting. Contractors embed a branded widget on their website — customers describe projects via AI-powered conversation and receive transparent P50/P80 estimates with cost breakdowns. Uses Reference Class Forecasting (RCF) methodology combined with LLM-powered narrative generation. Construction/home improvement MVP with a domain-agnostic backend.

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
- ✓ Tenant registration and management with email verification — v1.0
- ✓ JWT authentication with tenant_id claims and token management — v1.0
- ✓ BYOK Fernet encryption for OpenAI API keys with per-tenant HKDF derivation — v1.0
- ✓ Tenant isolation via TenantAwareCollection middleware — v1.0
- ✓ Per-tenant rate limiting by tier (trial/pro/enterprise) — v1.0
- ✓ MongoDB compound indexes for tenant isolation performance — v1.0
- ✓ OpenAI client with BYOK per-request injection and error classification — v1.0
- ✓ Git-based prompt management with immutable versioned JSON — v1.0
- ✓ LLM-generated estimate narratives streamed via SSE — v1.0
- ✓ Conversational scoping chat engine with follow-up questions and auto-trigger — v1.0
- ✓ LLM response caching by content hash — v1.0
- ✓ Shadow DOM widget container with CSS/JS isolation — v1.0
- ✓ Widget branding configuration (colors, logos, welcome text) — v1.0
- ✓ Conversational chat UI within widget — v1.0
- ✓ Lead capture form in widget flow — v1.0
- ✓ Estimate results display with P50/P80 ranges — v1.0
- ✓ Widget security hardening (DOMPurify XSS, auth error handling) — v1.0
- ✓ Widget analytics tracking (views, chat starts, estimate completions) — v1.0

### Active

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
- Offline mode — real-time LLM is core value

## Context

- **Project state**: v1.0 shipped — Epics 1-5 complete (foundation, RCF engine, multi-tenant, LLM integration, widget)
- **Codebase**: 18,517 LOC Python + 1,375 LOC TypeScript across 4 apps
- **Existing apps**: `apps/efofx-estimate/` (FastAPI backend), `apps/efofx-widget/` (React widget), `apps/estimator-mcp-functions/` (MCP serverless), `apps/synthetic-data-generator/`
- **Deployment**: DigitalOcean App Platform with auto-deploy
- **Database**: MongoDB Atlas with Motor async driver, TenantAwareCollection isolation
- **Synthetic data**: ~100 construction reference classes seeded
- **PRD**: Comprehensive PRD at `docs/PRD.md` with 7 epics, 44-51 stories
- **Architecture**: Detailed architecture doc at `docs/architecture.md`
- **Known tech debt**: INT-04 (EstimationSession tenant_id type), INT-05 (missing widget indexes), per-process LLM cache, deprecated collection accessors

## Constraints

- **Tech stack**: Python FastAPI backend, React/TypeScript widget, MongoDB Atlas — established in Epic 1
- **LLM provider**: OpenAI only for MVP (BYOK model)
- **Scale target**: 15 active tenants, 100k estimates/month, 99.5% uptime
- **Security**: Zero cross-tenant data leakage — hard isolation via TenantAwareCollection
- **Deployment**: DigitalOcean App Platform (already configured)
- **Domain**: Construction/home improvement MVP, IT/dev fast follow

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Domain-agnostic backend with flexible attributes | Support multiple verticals without schema changes | ✓ Good — proven in Epic 2 |
| Synthetic data with statistical distributions | Bootstrap estimates before real feedback exists | ✓ Good — validated against HomeAdvisor data |
| Simple keyword matching for RCF | Ship fast, upgrade to ML/NLP later | ✓ Good — sufficient for MVP |
| Shadow DOM for widget isolation | Prevent CSS/JS conflicts on contractor sites | ✓ Good — `?inline` CSS injection works, `all: initial` resets host styles |
| BYOK for OpenAI keys | Tenants control their own LLM costs | ✓ Good — 402 gate prevents platform cost exposure |
| Fernet encryption for stored API keys | Symmetric encryption sufficient for single-server MVP | ✓ Good — per-tenant HKDF derivation limits blast radius |
| JWT with tenant_id claims | Stateless auth with built-in tenant context | ✓ Good — dual auth (JWT + API key) works cleanly |
| TenantAwareCollection wrapper | Auto-inject tenant_id on all MongoDB operations | ✓ Good — eliminated all cross-tenant leak vectors |
| Per-operation collection instantiation | Avoid tenant_id drift across request lifecycle | ✓ Good — no stale tenant context bugs |
| OpenAI structured output (beta.chat.completions.parse) | Type-safe LLM responses via Pydantic | ✓ Good — no free-form text parsing needed |
| SSE via fetch+ReadableStream (not EventSource) | EventSource cannot set Authorization headers | ✓ Good — bearer-token-protected streams work |
| ScopingContext extraction via regex (not LLM) | Fast, cost-free readiness detection | ✓ Good — keyword patterns sufficient for 4 required fields |
| slowapi + custom header middleware | slowapi headers_enabled incompatible with Pydantic responses | ✓ Good — X-RateLimit-* headers injected reliably |
| Per-process LLM response cache | Simplest cache for single-worker deployments | ⚠️ Revisit — needs Valkey for multi-worker |

---
*Last updated: 2026-02-28 after v1.0 milestone*
