# Milestones

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

