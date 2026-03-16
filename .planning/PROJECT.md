# efOfX Estimation Service

## What This Is

An AI-powered, multi-tenant SaaS platform that transforms project estimation from guesswork into transparent, data-driven forecasting. Contractors embed a branded widget on their website — customers describe projects via AI-powered conversation and receive transparent P50/P80 estimates with cost breakdowns. After estimates, customers submit actual outcomes via email magic links, and contractors track estimate accuracy on a calibration dashboard. Uses Reference Class Forecasting (RCF) methodology combined with LLM-powered narrative generation. Construction/home improvement MVP with shared libraries ready for a second vertical.

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
- ✓ All v1.0 tech debt fixed (tenant_id type, widget indexes, deprecated accessors, ConsultationCTA, dependency sync) — v1.1
- ✓ Distributed Valkey cache replacing per-process LLM cache with tenant-scoped keys and graceful fallback — v1.1
- ✓ Customer feedback via email magic links (Resend SDK, SHA-256 tokens, two-step validation, immutable snapshots) — v1.1
- ✓ Contractor calibration dashboard with accuracy metrics, threshold enforcement, tenant-scoped aggregation — v1.1
- ✓ Shared Python package (efofx-shared: crypto, enums) with CI isolation test — v1.1
- ✓ Shared React component library (@efofx/ui: 5 components) with npm workspaces — v1.1
- ✓ Code quality standards documented and audited (STANDARDS.md, black + flake8 conformance) — v1.1

### Active

(None — next milestone requirements defined via `/gsd:new-milestone`)

### Out of Scope

- Communication coaching UI (internal/external narrative toggle) — post-MVP
- Multiple LLM provider support — OpenAI BYOK sufficient for MVP
- Real-time collaboration — async estimation is fine
- Mobile apps — web widget + API sufficient
- ML/NLP matching — simple keyword matching for MVP, upgrade later
- Offline mode — real-time LLM is core value
- Automated LLM prompt tuning — reliability disaster without human-reviewed evaluation harness
- Customer login accounts — doubles auth surface for one-time quote requestors; magic link is correct model
- Public-facing accuracy statistics — exposes calibration data that could undermine contractor relationships
- Email drip campaigns for non-responders — harms contractor-customer relationships; requires CAN-SPAM compliance
- Full Storybook documentation — over-engineered for 2-vertical extraction; TypeScript types + JSDoc sufficient
- Shared component npm registry publish — no external consumers; npm workspace protocol for internal sharing
- Automated reference class splitting/merging — insufficient sample size at v1.1 volumes

## Context

- **Project state**: v1.1 shipped — feedback loop closed, shared libraries extracted, ready for second vertical
- **Codebase**: ~20,850 LOC (18,437 Python + 2,410 TypeScript) across 4 apps + 2 shared packages
- **Apps**: `apps/efofx-estimate/` (FastAPI backend), `apps/efofx-widget/` (React widget), `apps/efofx-dashboard/` (React calibration dashboard), `apps/estimator-mcp-functions/` (MCP serverless), `apps/synthetic-data-generator/`
- **Packages**: `packages/efofx-shared/` (Python: crypto, enums), `packages/efofx-ui/` (React: ChatBubble, EstimateCard, TypingIndicator, ErrorBoundary, LoadingSkeleton)
- **Workspaces**: uv workspace (Python) + npm workspace (TypeScript)
- **Deployment**: DigitalOcean App Platform with auto-deploy, app.yaml with workspace-root source_dir
- **Database**: MongoDB Atlas with Motor async driver, TenantAwareCollection isolation
- **Cache**: Valkey (distributed, tenant-scoped keys, graceful fallback)
- **Email**: Resend SDK (transactional, graceful degradation when unconfigured)
- **CI**: GitHub Actions (Python isolation test + TypeScript build checks)
- **Synthetic data**: ~100 construction reference classes seeded, tagged with data_source: "synthetic"
- **PRD**: Comprehensive PRD at `docs/PRD.md` with 7 epics, 44-51 stories
- **Architecture**: Detailed architecture doc at `docs/architecture.md`

## Constraints

- **Tech stack**: Python FastAPI backend, React/TypeScript widget + dashboard, MongoDB Atlas — established in Epic 1
- **LLM provider**: OpenAI only for MVP (BYOK model)
- **Scale target**: 15 active tenants, 100k estimates/month, 99.5% uptime
- **Security**: Zero cross-tenant data leakage — hard isolation via TenantAwareCollection + tenant-scoped Valkey cache + tenant-scoped $lookup
- **Deployment**: DigitalOcean App Platform (already configured)
- **Domain**: Construction/home improvement MVP, IT/dev fast follow via shared libraries

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Domain-agnostic backend with flexible attributes | Support multiple verticals without schema changes | ✓ Good — proven in Epic 2 |
| Synthetic data with statistical distributions | Bootstrap estimates before real feedback exists | ✓ Good — validated against HomeAdvisor data |
| Simple keyword matching for RCF | Ship fast, upgrade to ML/NLP later | ✓ Good — sufficient for MVP |
| Shadow DOM for widget isolation | Prevent CSS/JS conflicts on contractor sites | ✓ Good — `?inline` CSS injection works, `all: initial` resets host styles |
| BYOK for OpenAI keys | Tenants control their own LLM costs | ✓ Good — 402 gate prevents platform cost exposure |
| Fernet encryption for stored API keys | Symmetric encryption sufficient for single-server MVP | ✓ Good — per-tenant HKDF derivation limits blast radius, extracted to shared package |
| JWT with tenant_id claims | Stateless auth with built-in tenant context | ✓ Good — dual auth (JWT + API key) works cleanly |
| TenantAwareCollection wrapper | Auto-inject tenant_id on all MongoDB operations | ✓ Good — eliminated all cross-tenant leak vectors |
| Per-operation collection instantiation | Avoid tenant_id drift across request lifecycle | ✓ Good — no stale tenant context bugs |
| OpenAI structured output (beta.chat.completions.parse) | Type-safe LLM responses via Pydantic | ✓ Good — no free-form text parsing needed |
| SSE via fetch+ReadableStream (not EventSource) | EventSource cannot set Authorization headers | ✓ Good — bearer-token-protected streams work |
| ScopingContext extraction via regex (not LLM) | Fast, cost-free readiness detection | ✓ Good — keyword patterns sufficient for 4 required fields |
| slowapi + custom header middleware | slowapi headers_enabled incompatible with Pydantic responses | ✓ Good — X-RateLimit-* headers injected reliably |
| Distributed Valkey cache (replacing per-process dict) | Multi-worker cache sharing, tenant isolation | ✓ Good — graceful fallback eliminates single-point-of-failure |
| Resend SDK for transactional email | Modern API, good deliverability, simple integration | ✓ Good — graceful degradation when unconfigured |
| Magic link with SHA-256 hashed storage | Database compromise cannot replay tokens | ✓ Good — two-step validation prevents scanner consumption |
| Immutable EstimateSnapshot at feedback submit | Feedback data not affected by later estimate changes | ✓ Good — copy-on-write at POST time |
| Calibration threshold of 10 real outcomes | Prevent misleading metrics from small samples | ✓ Good — ThresholdProgress shows clear "X more needed" |
| CSS custom properties (no Tailwind) for dashboard | Stripe/Linear muted aesthetic per user preference | ✓ Good — clean, data-focused design |
| uv + npm workspaces for shared packages | Monorepo without build complexity | ✓ Good — source imports work, CI validates isolation |
| Re-export shims for backward compatibility | Zero caller changes when extracting to shared packages | ✓ Good — 18+ callsite files unchanged |

---
*Last updated: 2026-03-16 after v1.1 milestone completed*
