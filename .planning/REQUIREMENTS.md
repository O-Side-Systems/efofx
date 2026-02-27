# Requirements: efOfX Estimation Service

**Defined:** 2026-02-26
**Core Value:** Trust through transparency — probabilistic estimates with explainable breakdowns that build contractor credibility with customers

## v1 Requirements

Requirements for Epics 3-7. Each maps to roadmap phases.

### Prerequisites

- [x] **PRQT-01**: Abandoned dependencies replaced (python-jose → PyJWT, passlib → pwdlib, openai v1 → v2)
- [x] **PRQT-02**: Cross-tenant data leak in rcf_engine.py fixed (tenant_id filtering on all queries)
- [x] **PRQT-03**: DB_COLLECTIONS import NameError in security.py fixed
- [x] **PRQT-04**: LLM parsing stub replaced with real OpenAI structured output
- [x] **PRQT-05**: Python runtime bumped to 3.11 in DO App Platform config

### Authentication & Tenancy

- [x] **AUTH-01**: Contractor can register for an efOfX account with company name, email, and password
- [x] **AUTH-02**: Contractor receives email verification after registration
- [x] **AUTH-03**: Contractor can log in with email/password and receive JWT access + refresh tokens
- [x] **AUTH-04**: JWT tokens contain tenant_id, user_id, and role claims with configurable expiration
- [x] **AUTH-05**: All protected API endpoints require valid JWT and extract tenant_id automatically
- [x] **AUTH-06**: Contractor can update profile settings (name, branding, tier)
- [x] **AUTH-07**: API key generated at registration (shown once, stored as bcrypt hash)

### Tenant Isolation

- [x] **ISOL-01**: Tenant isolation middleware enforces tenant_id on every MongoDB query automatically
- [x] **ISOL-02**: Zero cross-tenant data leakage — no query can return another tenant's data
- [x] **ISOL-03**: MongoDB compound indexes include tenant_id as first field for performance
- [x] **ISOL-04**: Platform-provided data (synthetic reference classes) accessible by all tenants

### BYOK Encryption

- [x] **BYOK-01**: Contractor can store their OpenAI API key encrypted with per-tenant derived Fernet key
- [x] **BYOK-02**: Encrypted keys are decrypted per-request for LLM calls (never stored in plaintext)
- [x] **BYOK-03**: Contractor can rotate their OpenAI key without re-registration
- [x] **BYOK-04**: LLM endpoints return 402 when no BYOK OpenAI key is stored (no platform fallback)

### Rate Limiting

- [x] **RATE-01**: Per-tenant rate limiting enforced based on tier (trial/pro/enterprise)
- [x] **RATE-02**: Rate limit headers returned in API responses (remaining, reset time)
- [x] **RATE-03**: Login endpoint rate limited (5 attempts per 15 minutes per IP)

### LLM Integration

- [x] **LLM-01**: OpenAI client instantiated per-request with tenant's decrypted BYOK key
- [ ] **LLM-02**: LLM responses streamed to client via Server-Sent Events (SSE)
- [x] **LLM-03**: Graceful handling of OpenAI API failures (timeouts, rate limits, key exhaustion)
- [x] **LLM-04**: LLM response caching by content hash for repeated identical queries

### Prompt Management

- [ ] **PRMT-01**: Prompts stored as versioned JSON files in git-tracked config/prompts/ directory
- [ ] **PRMT-02**: Each estimate records which prompt_version was used for traceability
- [ ] **PRMT-03**: Prompt versions are immutable once published (new version for changes)

### Conversational Scoping

- [ ] **CHAT-01**: User can describe their project through multi-turn chat conversation
- [ ] **CHAT-02**: Chat session persists conversation history within active session (MongoDB with TTL)
- [ ] **CHAT-03**: System determines when sufficient detail exists to generate estimate
- [ ] **CHAT-04**: System asks targeted follow-up questions to gather missing project details
- [ ] **CHAT-05**: Estimate generation triggered automatically or by user when ready

### Narrative Generation

- [ ] **NARR-01**: LLM generates human-readable narrative explaining estimate ranges and assumptions
- [ ] **NARR-02**: Narrative includes P50/P80 cost and timeline ranges with plain-language explanation
- [ ] **NARR-03**: Narrative references specific cost breakdown categories and adjustment factors
- [ ] **NARR-04**: "Thinking" state indicator shown while LLM generates narrative

### Widget Container

- [ ] **WDGT-01**: Widget renders inside Shadow DOM with closed mode for style/script isolation
- [ ] **WDGT-02**: Widget embeds on any site with single `<script>` tag (<5 lines of code)
- [ ] **WDGT-03**: Widget is mobile-responsive (sidebar, modal, full-width layouts)
- [ ] **WDGT-04**: Widget loads without visible "Powered by efOfX" branding (true white-label)
- [ ] **WDGT-05**: CORS configured per-tenant for widget API calls from contractor domains

### Widget Branding

- [ ] **BRND-01**: Contractor can configure widget colors (primary, secondary, accent)
- [ ] **BRND-02**: Contractor can set company logo URL displayed in widget header
- [ ] **BRND-03**: Contractor can customize widget button text and welcome message
- [ ] **BRND-04**: Branding config fetched via unauthenticated API endpoint (rate-limited by IP)

### Widget Features

- [ ] **WFTR-01**: Conversational chat UI within widget for project scoping
- [ ] **WFTR-02**: Lead capture form collects prospect email and phone before estimate
- [ ] **WFTR-03**: Estimate results displayed in widget with P50/P80 ranges and cost breakdown
- [ ] **WFTR-04**: Widget analytics track views, chat starts, estimate completions per tenant

### Widget Security

- [ ] **WSEC-01**: Widget JavaScript wrapped in global error boundary (no host page crashes)
- [ ] **WSEC-02**: All widget API calls authenticated via tenant API key
- [ ] **WSEC-03**: Widget input sanitized against XSS attacks

### Feedback Collection

- [ ] **FDBK-01**: Customer receives magic link email after project completion to submit actual costs
- [ ] **FDBK-02**: Magic link token is cryptographically random, single-use, with 7-day expiration
- [ ] **FDBK-03**: Customer feedback form captures actual cost, actual timeline, satisfaction rating
- [ ] **FDBK-04**: Contractor feedback dashboard shows all estimates with outcome submission status
- [ ] **FDBK-05**: Contractor can submit structured feedback on discrepancies (scope creep, market change, etc.)

### Calibration

- [ ] **CALB-01**: Variance calculated per estimate (actual vs estimated, as percentage)
- [ ] **CALB-02**: Tenant calibration dashboard shows aggregate accuracy metrics (mean variance, % within 20%)
- [ ] **CALB-03**: Calibration metrics exclude synthetic-only data (real outcomes only)
- [ ] **CALB-04**: Synthetic reference classes tagged with data_source field via migration

### Calibration Tuning

- [ ] **TUNE-01**: Synthetic data distributions tuned when real outcome patterns diverge significantly
- [ ] **TUNE-02**: LLM prompts refined based on feedback patterns (human-reviewed, not automated)

### Code Quality

- [ ] **QUAL-01**: Shared backend utilities extracted from duplicated service code
- [ ] **QUAL-02**: Shared frontend components library created from widget code
- [ ] **QUAL-03**: YAGNI pass removes unused code, dead endpoints, and unused dependencies
- [ ] **QUAL-04**: Code quality standards documented (naming, patterns, testing expectations)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Communication

- **COMM-01**: Internal/external narrative toggle (two-layer model UI)
- **COMM-02**: Stakeholder-specific communication framing

### Enterprise

- **ENTR-01**: SSO (SAML/OAuth) for enterprise tenant login
- **ENTR-02**: Audit log query interface for tenant admins
- **ENTR-03**: Custom rate limit tiers for enterprise contracts

### Platform Expansion

- **EXPN-01**: IT/development domain reference classes and generators
- **EXPN-02**: Multiple LLM provider support (Anthropic, Gemini)
- **EXPN-03**: Automated RLHF fine-tuning from labeled feedback datasets

### Analytics

- **ANLT-01**: Advanced analytics dashboard with charts and trends
- **ANLT-02**: Calibration trend charts over time

## Out of Scope

| Feature | Reason |
|---------|--------|
| Real-time multi-user collaboration | WebSocket infrastructure disproportionate to MVP needs; estimation is async |
| Customer accounts and login | End customers are one-time quote requestors; magic links sufficient |
| Mobile native apps | Web widget + API sufficient for MVP contractors |
| Multiple LLM providers | OpenAI BYOK sufficient; abstraction layer adds complexity without value now |
| SSO enterprise login | 15-tenant contractor MVP doesn't need corporate SSO |
| Automated RLHF fine-tuning | Requires labeled datasets and training infra; premature without feedback volume |
| Widget iframe embedding | Shadow DOM strictly better for style isolation, mobile UX, and CSP compatibility |
| Communication coaching UI | Architecture supports it but UX complexity is high; defer to post-MVP |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PRQT-01 | Phase 1 | Complete |
| PRQT-02 | Phase 1 | Complete |
| PRQT-03 | Phase 1 | Complete |
| PRQT-04 | Phase 1 | Complete |
| PRQT-05 | Phase 1 | Complete |
| AUTH-01 | Phase 2 | Complete |
| AUTH-02 | Phase 2 | Complete |
| AUTH-03 | Phase 2 | Complete |
| AUTH-04 | Phase 2 | Complete |
| AUTH-05 | Phase 2 | Complete |
| AUTH-06 | Phase 2 | Complete |
| AUTH-07 | Phase 2 | Complete |
| ISOL-01 | Phase 2 | Complete |
| ISOL-02 | Phase 2 | Complete |
| ISOL-03 | Phase 2 | Complete |
| ISOL-04 | Phase 2 | Complete |
| BYOK-01 | Phase 2 | Complete |
| BYOK-02 | Phase 2 | Complete |
| BYOK-03 | Phase 2 | Complete |
| BYOK-04 | Phase 2 | Complete |
| RATE-01 | Phase 2 | Complete |
| RATE-02 | Phase 2 | Complete |
| RATE-03 | Phase 2 | Complete |
| LLM-01 | Phase 3 | Complete |
| LLM-02 | Phase 3 | Pending |
| LLM-03 | Phase 3 | Complete |
| LLM-04 | Phase 3 | Complete |
| PRMT-01 | Phase 3 | Pending |
| PRMT-02 | Phase 3 | Pending |
| PRMT-03 | Phase 3 | Pending |
| CHAT-01 | Phase 3 | Pending |
| CHAT-02 | Phase 3 | Pending |
| CHAT-03 | Phase 3 | Pending |
| CHAT-04 | Phase 3 | Pending |
| CHAT-05 | Phase 3 | Pending |
| NARR-01 | Phase 3 | Pending |
| NARR-02 | Phase 3 | Pending |
| NARR-03 | Phase 3 | Pending |
| NARR-04 | Phase 3 | Pending |
| WDGT-01 | Phase 4 | Pending |
| WDGT-02 | Phase 4 | Pending |
| WDGT-03 | Phase 4 | Pending |
| WDGT-04 | Phase 4 | Pending |
| WDGT-05 | Phase 4 | Pending |
| BRND-01 | Phase 4 | Pending |
| BRND-02 | Phase 4 | Pending |
| BRND-03 | Phase 4 | Pending |
| BRND-04 | Phase 4 | Pending |
| WFTR-01 | Phase 4 | Pending |
| WFTR-02 | Phase 4 | Pending |
| WFTR-03 | Phase 4 | Pending |
| WFTR-04 | Phase 4 | Pending |
| WSEC-01 | Phase 4 | Pending |
| WSEC-02 | Phase 4 | Pending |
| WSEC-03 | Phase 4 | Pending |
| FDBK-01 | Phase 5 | Pending |
| FDBK-02 | Phase 5 | Pending |
| FDBK-03 | Phase 5 | Pending |
| FDBK-04 | Phase 5 | Pending |
| FDBK-05 | Phase 5 | Pending |
| CALB-01 | Phase 5 | Pending |
| CALB-02 | Phase 5 | Pending |
| CALB-03 | Phase 5 | Pending |
| CALB-04 | Phase 5 | Pending |
| TUNE-01 | Phase 5 | Pending |
| TUNE-02 | Phase 5 | Pending |
| QUAL-01 | Phase 6 | Pending |
| QUAL-02 | Phase 6 | Pending |
| QUAL-03 | Phase 6 | Pending |
| QUAL-04 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 70 total (5 prerequisites + 65 feature requirements)
- Mapped to phases: 70
- Unmapped: 0

---
*Requirements defined: 2026-02-26*
*Last updated: 2026-02-26 after 01-01 plan — PRQT-02 and PRQT-03 marked complete*
