# Milestones

## v1.1 Feedback & Quality (Shipped: 2026-03-16)

**Phases completed:** 5 phases (5-9), 15 plans
**Requirements:** 27/27 satisfied
**Timeline:** 16 days (2026-02-28 → 2026-03-15)
**Git range:** d20bfb6..582a7b0 — 69 commits
**LOC:** 20,847 Python + TypeScript (18,437 Python + 2,410 TypeScript)

**Key accomplishments:**
1. Fixed all v1.0 audit bugs — tenant_id type mismatch, missing widget indexes, deprecated accessors, dead code, dependency sync, ConsultationCTA wired to inline form with i18n
2. Replaced broken per-process LLM cache with distributed Valkey — tenant-scoped keys, graceful fallback on outage, warning cooldown
3. Built feedback email system with magic links — Resend SDK, SHA-256 hashed tokens with 72h TTL, two-step validation (GET idempotent, POST consumes), immutable estimate snapshots
4. Created calibration dashboard — CalibrationService with tenant-scoped $lookup aggregation, 10-outcome minimum threshold, React 19 + Recharts SPA with accuracy metrics, trend lines, sortable reference class table
5. Extracted shared libraries — packages/efofx-shared (Python: crypto, enums) and packages/efofx-ui (React: 5 components), uv + npm workspaces, CI isolation tests, STANDARDS.md with full conformance audit

**Delivered:** A self-improving estimation platform — contractors trigger feedback emails after estimates, customers submit actual outcomes via magic links, and calibration dashboard shows estimate accuracy against real data. Shared libraries enable second vertical without code duplication.

**Known Tech Debt (from audit):**
- Deployment config: VALKEY_URL, RESEND_API_KEY, APP_BASE_URL need to be set in DigitalOcean dashboard (env vars added to app.yaml in quick task 1)
- 13 human verification items across 5 phases (visual form rendering, email delivery, dashboard UX, CI green run, DO deployment)

**Audit:** See milestones/v1.1-MILESTONE-AUDIT.md

---

## v1.0 MVP (Shipped: 2026-02-28)

**Phases completed:** 5 phases (1-4.1), 18 plans, ~36 tasks
**Requirements:** 55/55 satisfied
**Timeline:** 2 days (2026-02-26 → 2026-02-27)
**Git range:** fix(01-01)..docs(v1.0) — 77 commits, 276 files changed
**LOC:** 18,517 Python + 1,375 TypeScript

**Key accomplishments:**
1. Fixed critical bugs and replaced abandoned deps (python-jose → PyJWT, passlib → pwdlib, openai v1 → v2)
2. Built complete multi-tenant SaaS foundation — registration, JWT auth, tenant isolation via TenantAwareCollection, BYOK Fernet encryption, per-tier rate limiting
3. Integrated real OpenAI LLM with BYOK per-request injection, prompt versioning, response caching, and SSE streaming
4. Built conversational scoping engine — multi-turn chat with follow-up questions and auto-trigger estimation
5. Shipped white-label embeddable widget — Shadow DOM isolation, branding config, chat UI, lead capture, estimate display
6. Closed integration gaps via Phase 4.1 — fixed tenant accessor bugs, wired index creation to startup, removed legacy dead code

**Delivered:** A commercially deployable multi-tenant estimation SaaS — contractors embed a branded widget, customers describe projects via AI-powered chat, and receive transparent P50/P80 estimates with cost breakdowns.

**Known Tech Debt (from audit):**
- INT-04: EstimationSession created with random PyObjectId() instead of tenant.tenant_id (mitigated by TenantAwareCollection insert stamp)
- INT-05: widget_analytics and widget_leads collections missing from create_indexes()
- LLM response cache is per-process only (no distributed cache)
- ConsultationCTA button destination not wired (logs to console)
- 5 deprecated collection accessors still in mongodb.py

**Audit:** See milestones/v1.0-MILESTONE-AUDIT.md

---

