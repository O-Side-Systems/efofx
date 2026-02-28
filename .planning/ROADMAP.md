# Roadmap: efOfX Estimation Service

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4.1 (shipped 2026-02-28)
- 📋 **v1.1** — Phases 5-6 (planned)

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

### 📋 Next Milestone (Planned)

- [ ] **Phase 5: Feedback and Calibration** — Magic link feedback collection, variance tracking, and calibration dashboard (Epic 6)
- [ ] **Phase 6: Code Quality and Hardening** — Extract shared utilities, remove dead code, document patterns (Epic 7)

## Phase Details

### Phase 5: Feedback and Calibration
**Goal**: The system can measure its own accuracy — customers submit actual costs via magic link, contractors see variance data, and the feedback loop that differentiates efOfX begins accumulating real signal
**Depends on**: Phase 4 (v1.0)
**Requirements**: FDBK-01, FDBK-02, FDBK-03, FDBK-04, FDBK-05, CALB-01, CALB-02, CALB-03, CALB-04, TUNE-01, TUNE-02
**Success Criteria** (what must be TRUE):
  1. After a project completes, the customer receives a magic link email — clicking it opens a feedback form without requiring a customer login
  2. Each magic link works exactly once and expires after 7 days — a used or expired link returns a clear error
  3. A contractor's calibration dashboard shows mean variance and percentage of estimates within 20% of actual — only real outcomes are counted, not synthetic data
  4. All existing synthetic reference classes are tagged with data_source: "synthetic" — the calibration aggregation query explicitly filters them out
  5. A contractor can submit structured feedback on a specific estimate discrepancy (scope creep, market change, etc.)
**Plans**: 4 plans

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
**Plans**: 3 plans

Plans:
- [ ] 06-01: Extract shared backend utilities and shared frontend components library (QUAL-01, QUAL-02)
- [ ] 06-02: YAGNI pass — remove unused code, dead endpoints, unused dependencies, and unimplemented stubs (QUAL-03)
- [ ] 06-03: Document code quality standards and audit existing code for conformance (QUAL-04)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Prerequisites | v1.0 | 2/2 | Complete | 2026-02-26 |
| 2. Multi-Tenant Foundation | v1.0 | 7/7 | Complete | 2026-02-27 |
| 3. LLM Integration | v1.0 | 4/4 | Complete | 2026-02-27 |
| 4. White-Label Widget | v1.0 | 4/4 | Complete | 2026-02-27 |
| 4.1 Integration Gap Closure | v1.0 | 1/1 | Complete | 2026-02-27 |
| 5. Feedback and Calibration | — | 0/4 | Not started | - |
| 6. Code Quality and Hardening | — | 0/3 | Not started | - |
