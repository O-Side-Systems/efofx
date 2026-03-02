# Roadmap: efOfX Estimation Service

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4.1 (shipped 2026-02-28)
- 🚧 **v1.1 Feedback & Quality** — Phases 5-9 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-4.1) — SHIPPED 2026-02-28</summary>

- [x] Phase 1: Prerequisites (2/2 plans) — completed 2026-02-26
- [x] Phase 2: Multi-Tenant Foundation (7/7 plans) — completed 2026-02-27
- [x] Phase 3: LLM Integration (4/4 plans) — completed 2026-02-27
- [x] Phase 4: White-Label Widget (4/4 plans) — completed 2026-02-27
- [x] Phase 4.1: Integration Gap Closure (1/1 plan) — completed 2026-02-27

See: milestones/v1.0-ROADMAP.md for full details

</details>

### 🚧 v1.1 Feedback & Quality (In Progress)

**Milestone Goal:** Close the feedback loop so estimates self-improve and clean up the codebase for a second vertical.

- [x] **Phase 5: Tech Debt & Foundation Cleanup** - Fix all v1.0 audit items before adding features (complete — 3/3 plans done)
- [x] **Phase 6: Valkey Infrastructure** - Replace broken per-process LLM cache with distributed Valkey (completed 2026-03-01)
- [ ] **Phase 7: Feedback Email & Magic Links** - Customers submit actual project outcomes via email magic link
- [ ] **Phase 8: Calibration Dashboard** - Contractors see historical estimate accuracy against real outcomes
- [ ] **Phase 9: Shared Library Extraction** - Extract shared packages to enable second vertical

## Phase Details

### Phase 5: Tech Debt & Foundation Cleanup
**Goal**: The codebase is correct and clean — all v1.0 audit bugs are fixed, deprecated patterns are removed, dead code is gone, and the foundation is solid for feature work
**Depends on**: Phase 4.1 (v1.0)
**Requirements**: DEBT-01, DEBT-02, DEBT-03, DEBT-04, DEBT-05, DEBT-06
**Success Criteria** (what must be TRUE):
  1. EstimationSession.tenant_id stores the correct tenant identifier type — no silent type mismatch between creation and TenantAwareCollection filter
  2. widget_analytics and widget_leads compound indexes exist in the database — the analytics collections are not doing full-collection scans
  3. All 5 deprecated collection accessors are removed from mongodb.py — no code references them
  4. The ConsultationCTA button navigates to a real destination — it does not log to console or throw a JavaScript error
  5. requirements.txt and pyproject.toml declare the same dependencies — no production dependency is missing from requirements.txt
**Plans**: 3 plans (Wave 1 parallel + Wave 2 gap closure)

Plans:
- [x] 05-01: Fix tenant_id type + migration (DEBT-01), add widget indexes (DEBT-02), remove deprecated accessors (DEBT-03), delete dead code (DEBT-05), sync requirements.txt (DEBT-06)
- [x] 05-02: Wire ConsultationCTA to inline contact form with backend endpoint and email notification (DEBT-04)
- [x] 05-03: Gap closure — wire locale and consultation_form_labels through branding API response (DEBT-04)

### Phase 6: Valkey Infrastructure
**Goal**: LLM response caching works correctly across all Gunicorn workers — the per-process cache bug is gone, cache is tenant-scoped, and Valkey outages do not crash the service
**Depends on**: Phase 5
**Requirements**: INFR-01, INFR-02, INFR-03
**Success Criteria** (what must be TRUE):
  1. Two simultaneous workers serve the same LLM cache hit — a response cached by Worker A is returned by Worker B without a live LLM call
  2. Cache keys include tenant_id — a cached response for Tenant A cannot be served to Tenant B
  3. With Valkey unreachable, estimation requests complete successfully via live LLM call — no 500 errors, no user-visible cache errors
**Plans**: 1 plan (Wave 1)

Plans:
- [ ] 06-01: Create ValkeyCache service with tenant-scoped keys and graceful fallback, wire into LLMService replacing per-process dict cache, add unit tests (INFR-01, INFR-02, INFR-03)

### Phase 7: Feedback Email & Magic Links
**Goal**: Customers can submit actual project costs and outcomes via a time-limited email link after an estimate — no customer login required, and the data is stored against the estimate for calibration
**Depends on**: Phase 6
**Requirements**: FEED-01, FEED-02, FEED-03, FEED-04, FEED-05, FEED-06, FEED-07
**Success Criteria** (what must be TRUE):
  1. After an estimate is delivered, a contractor can trigger a feedback email — the customer receives a contextualized email showing the original P50/P80 range with a magic link CTA
  2. A customer clicks the magic link and sees a feedback form — no account creation required, and email security scanners following the link URL do not consume the token
  3. A customer submits actual cost, actual timeline, rating, and a discrepancy reason from a structured enum — the form accepts and stores the submission
  4. A submitted magic link cannot be used a second time — the form shows a "thank you" message on re-visit; an expired link shows a friendly expiry message
  5. The feedback document is stored with an immutable snapshot of the original estimate and a reference class linkage — the stored data is not affected by later estimate changes
**Plans**: 4 plans (Wave 1 parallel + Wave 2 parallel)

Plans:
- [ ] 07-01: Resend SDK install, RESEND_API_KEY config, FeedbackEmailService with graceful degradation (FEED-01) [Wave 1]
- [ ] 07-02: Feedback models (DiscrepancyReason, FeedbackMagicLink, FeedbackSubmission, EstimateSnapshot), MagicLinkService with TDD, MongoDB TTL indexes (FEED-02, FEED-03, FEED-07) [Wave 1]
- [ ] 07-03: Jinja2 email template with tenant branding + estimate context, POST /feedback/request-email trigger endpoint (FEED-04) [Wave 2, depends: 07-01, 07-02]
- [ ] 07-04: Feedback form HTML templates (form/expired/submitted), GET/POST /feedback/form/{token} endpoints, store_feedback_with_snapshot (FEED-05, FEED-06) [Wave 2, depends: 07-02]

### Phase 8: Calibration Dashboard
**Goal**: Contractors can see how accurate their estimates have been against real outcomes — accuracy metrics are displayed only when statistically meaningful, and synthetic data is never mixed into calibration calculations
**Depends on**: Phase 7
**Requirements**: CALB-01, CALB-02, CALB-03, CALB-04, CALB-05, CALB-06
**Success Criteria** (what must be TRUE):
  1. All existing synthetic reference class documents have data_source: "synthetic" — a calibration query run against the production database returns zero synthetic records in its result set
  2. A contractor with fewer than 10 real outcomes sees a progress indicator showing how many more outcomes are needed — no partial accuracy metrics are shown
  3. A contractor with 10 or more real outcomes sees mean variance, accuracy buckets (within 10/20/30% of actual), and per-reference-class breakdown in the dashboard
  4. The calibration dashboard is accessible to authenticated contractors at its own URL — it loads without errors and displays tenant-scoped data only
  5. The calibration aggregation pipeline explicitly filters tenant_id in every $lookup inner pipeline — a database query log shows no cross-tenant joins
**Plans**: TBD

Plans:
- [ ] 08-01: Tag existing synthetic reference class documents with data_source field migration (CALB-01)
- [ ] 08-02: CalibrationService — tenant-scoped MongoDB aggregation, accuracy metrics calculation, minimum threshold enforcement (CALB-02, CALB-03, CALB-04)
- [ ] 08-03: Scaffold apps/efofx-dashboard/ Vite + React 19 app with JWT auth and React Query (CALB-05)
- [ ] 08-04: Calibration dashboard UI — accuracy charts with Recharts, threshold progress indicator, per-reference-class breakdown (CALB-05, CALB-06)

### Phase 9: Shared Library Extraction
**Goal**: Shared backend utilities and shared frontend components live in workspace packages that any future vertical can consume — the IT/dev vertical can be initialized in v1.2 without copying code
**Depends on**: Phase 8
**Requirements**: EXTR-01, EXTR-02, EXTR-03, EXTR-04, EXTR-05
**Success Criteria** (what must be TRUE):
  1. A written boundary document exists specifying what belongs in packages/efofx-shared/ and packages/efofx-ui/ versus staying in apps/ — it resolves any ambiguous future extraction decisions
  2. packages/efofx-shared/ installs in a fresh Python virtualenv with only its declared dependencies — no FastAPI, Motor, or apps/ imports leak in
  3. packages/efofx-ui/ contains EstimateCard, ChatBubble, and TypingIndicator with no widget-specific state or estimation-domain logic
  4. A CI test runs on every PR and fails if the shared Python package cannot be imported in a fresh environment — the circular import protection is automated
  5. Code quality standards are documented and the existing codebase is audited for conformance
**Plans**: TBD

Plans:
- [ ] 09-01: Write shared library boundary document (EXTR-01)
- [ ] 09-02: Extract packages/efofx-shared/ Python package with uv workspace, add CI isolation test (EXTR-02, EXTR-04)
- [ ] 09-03: Extract packages/efofx-ui/ React components with npm workspaces (EXTR-03)
- [ ] 09-04: Code quality standards documentation and codebase conformance audit (EXTR-05)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Prerequisites | v1.0 | 2/2 | Complete | 2026-02-26 |
| 2. Multi-Tenant Foundation | v1.0 | 7/7 | Complete | 2026-02-27 |
| 3. LLM Integration | v1.0 | 4/4 | Complete | 2026-02-27 |
| 4. White-Label Widget | v1.0 | 4/4 | Complete | 2026-02-27 |
| 4.1 Integration Gap Closure | v1.0 | 1/1 | Complete | 2026-02-27 |
| 5. Tech Debt & Foundation Cleanup | v1.1 | 3/3 | Complete | 2026-02-28 |
| 6. Valkey Infrastructure | 1/1 | Complete   | 2026-03-01 | - |
| 7. Feedback Email & Magic Links | 2/4 | In Progress|  | - |
| 8. Calibration Dashboard | v1.1 | 0/4 | Not started | - |
| 9. Shared Library Extraction | v1.1 | 0/4 | Not started | - |
