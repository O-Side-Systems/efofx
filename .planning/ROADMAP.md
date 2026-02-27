# Roadmap: efOfX Estimation Service (Epics 3-7)

## Overview

Epics 1-2 delivered a working RCF estimation engine. Epics 3-7 transform that engine into a commercially deployable multi-tenant SaaS product. The phases follow a hard dependency chain: prerequisites unblock infrastructure, multi-tenant infrastructure unblocks LLM integration, LLM integration unblocks the widget, the widget produces estimates that make feedback meaningful, and hardening comes last when there is real code to harden. No phase can be safely reordered.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Prerequisites** - Fix known bugs and replace abandoned dependencies before building anything new
- [x] **Phase 2: Multi-Tenant Foundation** - Tenant registration, JWT auth, hard isolation, BYOK encryption, and rate limiting (Epic 3) (completed 2026-02-27)
- [x] **Phase 3: LLM Integration** - Real OpenAI integration with BYOK, prompt versioning, streaming chat, and narrative generation (Epic 4) (completed 2026-02-27)
- [ ] **Phase 4: White-Label Widget** - Embeddable Shadow DOM widget with branding, chat UI, lead capture, and estimate display (Epic 5)
- [ ] **Phase 5: Feedback and Calibration** - Magic link feedback collection, variance tracking, and calibration dashboard (Epic 6)
- [ ] **Phase 6: Code Quality and Hardening** - Extract shared utilities, remove dead code, document patterns (Epic 7)

## Phase Details

### Phase 1: Prerequisites
**Goal**: All known bugs are fixed and abandoned dependencies replaced so Phase 2 can be built on a stable foundation
**Depends on**: Nothing (first phase)
**Requirements**: PRQT-01, PRQT-02, PRQT-03, PRQT-04, PRQT-05
**Success Criteria** (what must be TRUE):
  1. The backend starts without errors — no NameError on `DB_COLLECTIONS` in security.py
  2. API key authentication works end-to-end — a valid API key can be used to call a protected endpoint without a 500 error
  3. `requirements.txt` contains PyJWT, pwdlib, and openai>=2.20.0 with no references to python-jose or passlib
  4. The RCF engine `rcf_engine.py` filters all queries by `tenant_id` — a query for tenant A returns zero results from tenant B's data
  5. DigitalOcean App Platform is running Python 3.11 and the app deploys cleanly
**Plans**: 2 plans

Plans:
- [x] 01-01: Fix DB_COLLECTIONS NameError in security.py (PRQT-03) and cross-tenant query bug in rcf_engine.py with integration test (PRQT-02)
- [x] 01-02: Replace python-jose/passlib/openai v1 with PyJWT/pwdlib/openai v2 (PRQT-01), implement structured LLM output (PRQT-04), bump Python to 3.11 (PRQT-05)

### Phase 2: Multi-Tenant Foundation
**Goal**: Contractors can register, authenticate, and have their data completely isolated from other tenants — the security layer every subsequent feature depends on
**Depends on**: Phase 1
**Requirements**: AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05, AUTH-06, AUTH-07, ISOL-01, ISOL-02, ISOL-03, ISOL-04, BYOK-01, BYOK-02, BYOK-03, BYOK-04, RATE-01, RATE-02, RATE-03
**Success Criteria** (what must be TRUE):
  1. A contractor can register with company name, email, and password and receive a verification email
  2. A verified contractor can log in with email/password and receive JWT access and refresh tokens containing tenant_id, user_id, and role claims
  3. Calling any protected endpoint with an expired or missing JWT returns 401 — never returns data
  4. Two tenants cannot see each other's data — any query from tenant A returns zero results from tenant B regardless of what parameters are used
  5. A contractor can store their OpenAI API key and it is used for their LLM calls — the key is never stored in plaintext and can be rotated without re-registration
  6. A trial-tier tenant is rate-limited after exceeding their threshold — login is rate-limited to 5 attempts per 15 minutes per IP
**Plans**: 7 plans (5 original + 2 gap closure)

Plans:
- [ ] 02-01: Tenant registration, email verification, and profile management (AUTH-01, AUTH-02, AUTH-06, AUTH-07)
- [ ] 02-02: JWT authentication — token generation, refresh, and protected endpoint middleware (AUTH-03, AUTH-04, AUTH-05)
- [ ] 02-03: TenantAwareCollection wrapper and MongoDB compound indexes for hard isolation (ISOL-01, ISOL-02, ISOL-03, ISOL-04)
- [ ] 02-04: BYOK Fernet encryption with per-tenant HKDF derivation and trial fallback (BYOK-01, BYOK-02, BYOK-03, BYOK-04)
- [ ] 02-05: Per-tenant rate limiting with Valkey backend and login brute-force protection (RATE-01, RATE-02, RATE-03)
- [ ] 02-06: [GAP CLOSURE] Refactor tenant_service.py — replace deprecated collection accessors with TenantAwareCollection (ISOL-02)
- [ ] 02-07: [GAP CLOSURE] Fix BYOK-04 docs contradiction, wire LLMService to accept BYOK key (BYOK-02, BYOK-04)

### Phase 3: LLM Integration
**Goal**: Contractors and their customers can converse with the system about a project and receive a real AI-generated estimate narrative — no stubs, no hardcoded values
**Depends on**: Phase 2
**Requirements**: LLM-01, LLM-02, LLM-03, LLM-04, PRMT-01, PRMT-02, PRMT-03, CHAT-01, CHAT-02, CHAT-03, CHAT-04, CHAT-05, NARR-01, NARR-02, NARR-03, NARR-04
**Success Criteria** (what must be TRUE):
  1. Starting a project description in chat produces real AI responses streamed token-by-token via SSE — no full-response wait
  2. The system asks targeted follow-up questions and recognizes when enough detail exists to trigger estimate generation
  3. An estimate narrative includes plain-language P50/P80 ranges, cost breakdown categories, and adjustment factor references
  4. Each estimate record stores the prompt_version used so calibration can trace accuracy to specific prompt changes
  5. When the tenant's OpenAI key is exhausted or the API fails, the system returns a clear error — it never silently falls back to the platform key or returns a stub response
  6. Repeated identical queries return cached responses without making redundant OpenAI API calls
**Plans**: TBD

Plans:
- [ ] 03-01: OpenAI v2 BYOK client — per-request key injection, error handling, and response caching (LLM-01, LLM-03, LLM-04)
- [ ] 03-02: Git-versioned prompt registry — JSON prompts in config/prompts/, immutable versions, prompt_version on estimates (PRMT-01, PRMT-02, PRMT-03)
- [ ] 03-03: Conversational scoping engine — multi-turn chat with MongoDB TTL sessions, follow-up questions, and estimate trigger detection (CHAT-01, CHAT-02, CHAT-03, CHAT-04, CHAT-05)
- [ ] 03-04: Streaming SSE endpoint and narrative generation with real OpenAI structured output (LLM-02, NARR-01, NARR-02, NARR-03, NARR-04)

### Phase 4: White-Label Widget
**Goal**: A contractor can embed a single script tag on their website and their customers see a fully branded estimation chat experience that captures leads and displays real estimates
**Depends on**: Phase 3
**Requirements**: WDGT-01, WDGT-02, WDGT-03, WDGT-04, WDGT-05, BRND-01, BRND-02, BRND-03, BRND-04, WFTR-01, WFTR-02, WFTR-03, WFTR-04, WSEC-01, WSEC-02, WSEC-03
**Success Criteria** (what must be TRUE):
  1. Pasting a 5-line script tag onto any website loads the widget without breaking the host page — even on sites with jQuery 1.x or broken fetch
  2. The widget displays the contractor's logo, brand colors, and custom welcome message — no "Powered by efOfX" is visible
  3. A site visitor can describe their project through the chat, submit their email and phone, and receive a P50/P80 estimate with cost breakdown — all within the widget
  4. The widget renders correctly on mobile viewports and in sidebar, modal, and full-width layout configurations
  5. All widget API calls use the tenant API key — requests without a valid key are rejected
**Plans**: 4 plans

Plans:
- [ ] 04-01: Widget IIFE shell — Shadow DOM isolation, CSS injection, floating/inline modes, responsive layouts, error boundary (WDGT-01, WDGT-02, WDGT-03, WDGT-04, WSEC-01)
- [ ] 04-02: Backend branding API, per-tenant CORS middleware, widget models/service, lead capture endpoint (BRND-01, BRND-02, BRND-03, BRND-04, WDGT-05)
- [ ] 04-03: Chat UI components, lead capture form, estimate range visualization, SSE narrative streaming, consultation CTA (WFTR-01, WFTR-02, WFTR-03)
- [ ] 04-04: Widget security hardening — API key auth verification, XSS sanitization, analytics event tracking (WSEC-02, WSEC-03, WFTR-04)

### Phase 5: Feedback and Calibration
**Goal**: The system can measure its own accuracy — customers submit actual costs via magic link, contractors see variance data, and the feedback loop that differentiates efOfX begins accumulating real signal
**Depends on**: Phase 4
**Requirements**: FDBK-01, FDBK-02, FDBK-03, FDBK-04, FDBK-05, CALB-01, CALB-02, CALB-03, CALB-04, TUNE-01, TUNE-02
**Success Criteria** (what must be TRUE):
  1. After a project completes, the customer receives a magic link email — clicking it opens a feedback form without requiring a customer login
  2. Each magic link works exactly once and expires after 7 days — a used or expired link returns a clear error
  3. A contractor's calibration dashboard shows mean variance and percentage of estimates within 20% of actual — only real outcomes are counted, not synthetic data
  4. All existing synthetic reference classes are tagged with data_source: "synthetic" — the calibration aggregation query explicitly filters them out
  5. A contractor can submit structured feedback on a specific estimate discrepancy (scope creep, market change, etc.)
**Plans**: TBD

Plans:
- [ ] 05-01: Data source migration — tag all existing synthetic reference classes with data_source field (CALB-04)
- [ ] 05-02: Magic link feedback system — HMAC token generation, single-use enforcement, customer feedback form (FDBK-01, FDBK-02, FDBK-03)
- [ ] 05-03: Contractor feedback dashboard and structured discrepancy submission (FDBK-04, FDBK-05)
- [ ] 05-04: Calibration service — variance calculation, real-data-only aggregation, and calibration dashboard endpoint (CALB-01, CALB-02, CALB-03, TUNE-01, TUNE-02)

### Phase 6: Code Quality and Hardening
**Goal**: The codebase is clean, shared code lives in shared packages, dead code is removed, and patterns are documented so the next contributor (human or AI) can work efficiently
**Depends on**: Phase 5
**Requirements**: QUAL-01, QUAL-02, QUAL-03, QUAL-04
**Success Criteria** (what must be TRUE):
  1. Shared backend logic is in a shared utility module — no copy-pasted service code exists across apps
  2. The frontend has a shared component library — no duplicated widget components
  3. The codebase has no dead endpoints, unused dependencies, or unimplemented stubs — every file does something real
  4. Code quality standards are documented and the existing codebase conforms to them (naming conventions, error handling patterns, testing expectations)
**Plans**: TBD

Plans:
- [ ] 06-01: Extract shared backend utilities and shared frontend components library (QUAL-01, QUAL-02)
- [ ] 06-02: YAGNI pass — remove unused code, dead endpoints, unused dependencies, and unimplemented stubs (QUAL-03)
- [ ] 06-03: Document code quality standards and audit existing code for conformance (QUAL-04)

## Progress

**Execution Order:**
Phases execute in strict dependency order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Prerequisites | 2/2 | Complete | 2026-02-26 |
| 2. Multi-Tenant Foundation | 7/7 | Complete   | 2026-02-27 |
| 3. LLM Integration | 4/4 | Complete   | 2026-02-27 |
| 4. White-Label Widget | 1/4 | In Progress|  |
| 5. Feedback and Calibration | 0/4 | Not started | - |
| 6. Code Quality and Hardening | 0/3 | Not started | - |
