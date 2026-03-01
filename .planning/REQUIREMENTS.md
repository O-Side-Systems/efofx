# Requirements: efOfX Estimation Service

**Defined:** 2026-02-28
**Core Value:** Trust through transparency — probabilistic estimates with explainable breakdowns that build contractor credibility with customers

## v1.1 Requirements

Requirements for Feedback & Quality milestone. Each maps to roadmap phases.

### Tech Debt

- [x] **DEBT-01**: Fix EstimationSession tenant_id type to match TenantAwareCollection expectations (INT-04)
- [x] **DEBT-02**: Add compound indexes for widget_analytics and widget_leads collections (INT-05)
- [x] **DEBT-03**: Remove 5 deprecated collection accessors from mongodb.py
- [x] **DEBT-04**: Wire ConsultationCTA button to contractor contact flow destination
- [x] **DEBT-05**: YAGNI pass — remove unused code paths and dead imports
- [x] **DEBT-06**: Sync requirements.txt with pyproject.toml (fastapi-mail, valkey)

### Infrastructure

- [ ] **INFR-01**: Replace per-process LLM dict cache with distributed Valkey cache
- [ ] **INFR-02**: Valkey cache keys prefixed with tenant_id to prevent cross-tenant collisions
- [ ] **INFR-03**: Graceful Valkey fallback — cache outage falls back to live LLM call, not 500

### Feedback

- [ ] **FEED-01**: Email infrastructure setup — transactional provider, SPF/DKIM/DMARC configuration
- [ ] **FEED-02**: Magic link token generation (secrets.token_urlsafe) with SHA-256 hashed storage and 72h TTL
- [ ] **FEED-03**: Two-step token validation — idempotent GET renders form, POST consumes token
- [ ] **FEED-04**: Contextualized feedback email with estimate range summary and CTA
- [ ] **FEED-05**: Customer feedback form with structured fields (actual_cost, actual_timeline, rating, discrepancy reason enum)
- [ ] **FEED-06**: Feedback document storage with immutable estimate snapshot and reference class linkage
- [ ] **FEED-07**: Graceful token states — valid (form), expired (friendly message), used (thank you)

### Calibration

- [ ] **CALB-01**: Tag existing synthetic reference classes with data_source: "synthetic"
- [ ] **CALB-02**: Calibration metrics API — mean variance, accuracy buckets (10/20/30%), per-reference-class breakdown
- [ ] **CALB-03**: Minimum 10 real outcome threshold enforced before displaying any metrics
- [ ] **CALB-04**: Tenant-scoped $lookup aggregation with explicit tenant_id in inner pipeline
- [ ] **CALB-05**: Calibration dashboard app (apps/efofx-dashboard/) with Recharts charts
- [ ] **CALB-06**: Dashboard shows progress indicator below minimum threshold ("X more outcomes needed")

### Extraction

- [ ] **EXTR-01**: Shared library boundary document — what goes in shared vs stays in apps
- [ ] **EXTR-02**: Extract packages/efofx-shared/ Python package (crypto, validation, calculation utils) with uv workspace
- [ ] **EXTR-03**: Extract packages/efofx-ui/ React components (EstimateCard, ChatBubble, TypingIndicator) with npm workspaces
- [ ] **EXTR-04**: CI test verifying shared packages install in fresh env with zero app imports
- [ ] **EXTR-05**: Code quality standards documentation

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Feedback Automation

- **FAUTO-01**: Automated contractor notification after customer feedback submission
- **FAUTO-02**: Email drip campaign for non-responders (requires CAN-SPAM compliance)

### Advanced Calibration

- **ACALB-01**: Temporal calibration trend charts (rolling 30/90-day accuracy)
- **ACALB-02**: Synthetic data weight adjustment workflow (semi-automated)
- **ACALB-03**: Per-estimate drill-down view for contractors

### Second Vertical

- **VERT-01**: IT/dev vertical reference classes using shared library
- **VERT-02**: Vertical-specific prompt templates

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Automated LLM prompt tuning | Reliability disaster without human-reviewed evaluation harness |
| Customer login accounts | Doubles auth surface for one-time quote requestors; magic link is correct model |
| Public-facing accuracy statistics | Exposes calibration data that could undermine contractor relationships |
| Email drip campaigns for non-responders | Harms contractor-customer relationships; requires CAN-SPAM compliance |
| Full Storybook documentation | Over-engineered for 2-vertical extraction; TypeScript types + JSDoc sufficient |
| Shared component npm registry publish | No external consumers; pnpm workspace protocol for internal sharing |
| Automated reference class splitting/merging | Insufficient sample size at v1.1 volumes; noise chasing |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DEBT-01 | Phase 5 | Complete |
| DEBT-02 | Phase 5 | Complete |
| DEBT-03 | Phase 5 | Complete |
| DEBT-04 | Phase 5 | Complete |
| DEBT-05 | Phase 5 | Complete |
| DEBT-06 | Phase 5 | Complete |
| INFR-01 | Phase 6 | Pending |
| INFR-02 | Phase 6 | Pending |
| INFR-03 | Phase 6 | Pending |
| FEED-01 | Phase 7 | Pending |
| FEED-02 | Phase 7 | Pending |
| FEED-03 | Phase 7 | Pending |
| FEED-04 | Phase 7 | Pending |
| FEED-05 | Phase 7 | Pending |
| FEED-06 | Phase 7 | Pending |
| FEED-07 | Phase 7 | Pending |
| CALB-01 | Phase 8 | Pending |
| CALB-02 | Phase 8 | Pending |
| CALB-03 | Phase 8 | Pending |
| CALB-04 | Phase 8 | Pending |
| CALB-05 | Phase 8 | Pending |
| CALB-06 | Phase 8 | Pending |
| EXTR-01 | Phase 9 | Pending |
| EXTR-02 | Phase 9 | Pending |
| EXTR-03 | Phase 9 | Pending |
| EXTR-04 | Phase 9 | Pending |
| EXTR-05 | Phase 9 | Pending |

**Coverage:**
- v1.1 requirements: 27 total
- Mapped to phases: 27
- Unmapped: 0

---
*Requirements defined: 2026-02-28*
*Last updated: 2026-02-28 after roadmap creation — all 27 requirements mapped*
