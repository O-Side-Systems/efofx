---
phase: 05-tech-debt-foundation-cleanup
verified: 2026-02-28T06:00:00Z
status: human_needed
score: 9/9 must-haves verified
re_verification: true
  previous_status: gaps_found
  previous_score: 8/9
  gaps_closed:
    - "Per-tenant locale and consultation_form_labels are propagated through the branding API response to the frontend"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Open the widget in a browser, click 'Request Free Consultation', verify the inline form appears (not a modal, not a redirect)"
    expected: "Name, email, phone, and message fields appear inline within the chat panel; filling and submitting the form shows a success confirmation in the same panel"
    why_human: "Visual rendering, panel containment, and post-submit UX state cannot be verified programmatically"
  - test: "Submit the consultation form and check the backend database"
    expected: "A document appears in widget_leads with lead_type='consultation', name, email, phone, message, session_id, and tenant_id fields"
    why_human: "Requires a live database and authenticated API key to confirm end-to-end persistence"
---

# Phase 5: Tech Debt Foundation Cleanup Verification Report

**Phase Goal:** The codebase is correct and clean — all v1.0 audit bugs are fixed, deprecated patterns are removed, dead code is gone, and the foundation is solid for feature work
**Verified:** 2026-02-28
**Status:** human_needed
**Re-verification:** Yes — after gap closure plan 05-03 (commit 8d58854)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | EstimationSession.tenant_id is typed as str and estimation_service.py passes tenant.tenant_id when constructing EstimationSession | VERIFIED | `estimation.py` line 67: `tenant_id: str = Field(..., description="Associated tenant ID (UUID string)")`. `estimation_service.py` line 179: `tenant_id=tenant.tenant_id` |
| 2 | widget_analytics has a unique compound index on (tenant_id, date) and widget_leads has compound indexes on (tenant_id, session_id) and (tenant_id, captured_at) | VERIFIED | `mongodb.py` lines 235-255: all 3 indexes present with correct fields; widget_analytics index has `unique=True` |
| 3 | All 5 deprecated collection accessors are deleted from mongodb.py and zero import references to them exist in the codebase | VERIFIED | Grep across entire `apps/efofx-estimate` directory returns zero matches for all 5 function names. `get_tenants_collection` at line 117 is a live non-deprecated accessor used by tenant_service.py — not in scope. |
| 4 | qa_epic2.py and test_count.py are deleted; EstimationRequest, EstimationResult, and CostBreakdown classes are deleted from estimation.py | VERIFIED | Both files confirmed absent. Grep across `apps/efofx-estimate/app/models/estimation.py` returns zero matches for all 3 class names. |
| 5 | requirements.txt contains fastapi-mail, valkey, slowapi, pwdlib[bcrypt], and pydantic[email] entries | VERIFIED | All 5 entries confirmed present in `requirements.txt`: fastapi-mail==1.6.2, valkey>=6.1.0, slowapi==0.1.9, pwdlib[bcrypt]==0.3.0, pydantic[email]>=2.11.0 |
| 6 | migrate_estimation_session_tenant_id() exists in mongodb.py and is called from lifespan() after create_indexes() | VERIFIED | Function at `mongodb.py` line 264. `main.py` line 22 imports it; line 39: `await migrate_estimation_session_tenant_id()  # DEBT-01` called after `create_indexes()` |
| 7 | Clicking the ConsultationCTA button reveals an inline contact form within the chat panel | VERIFIED | `ConsultationCTA.tsx` line 15: `showForm` state; lines 29-37: `if (showForm) { return <ConsultationForm ... /> }`. Console stub removed. |
| 8 | Submitting the contact form saves a document in widget_leads and sends email notification with graceful degradation | VERIFIED | `widget_service.py` lines 113-135: `save_consultation()` writes to widget_leads with `lead_type="consultation"`, then calls `_send_consultation_email()`; email skipped with warning log if MAIL_SERVER/MAIL_USERNAME not set |
| 9 | Form labels and placeholder text display in English by default, with Spanish translations bundled, and per-tenant overrides from branding config take priority | VERIFIED | **Gap closed.** `widget_service.py` lines 88-89: `locale=branding.locale` and `consultation_form_labels=branding.consultation_form_labels` added to `BrandingConfigResponse` constructor. Per-tenant values now propagate from stored `BrandingConfig` to the API response. Frontend `getLabels()` merge logic was already correct. Commit: 8d58854. |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `apps/efofx-estimate/app/models/estimation.py` | EstimationSession with tenant_id: str | VERIFIED | Line 67: `tenant_id: str = Field(...)` |
| `apps/efofx-estimate/app/db/mongodb.py` | create_indexes with widget_analytics and widget_leads indexes, migration function, no deprecated accessors | VERIFIED | Lines 235-255 (indexes), lines 264-289 (migration fn), zero deprecated accessor functions present |
| `apps/efofx-estimate/app/main.py` | Startup lifespan calling migration after create_indexes | VERIFIED | Line 22 imports `migrate_estimation_session_tenant_id`; line 39 calls it in lifespan |
| `apps/efofx-estimate/requirements.txt` | Complete dependency list matching pyproject.toml | VERIFIED | fastapi-mail==1.6.2, valkey>=6.1.0, slowapi==0.1.9, pwdlib[bcrypt]==0.3.0, pydantic[email]>=2.11.0 all present |
| `apps/efofx-widget/src/components/ConsultationForm.tsx` | Inline contact form component with locale support | VERIFIED | Substantive implementation: controlled inputs for name/email/phone/message, isSubmitting state, validation, error display, getLabels() usage |
| `apps/efofx-widget/src/i18n/consultationForm.ts` | en and es locale label constants | VERIFIED | CONSULTATION_FORM_LABELS with full en and es translations, getLabels() merge function with null-safe spread |
| `apps/efofx-estimate/app/api/widget.py` | POST /widget/consultation endpoint | VERIFIED | Lines 107-139: POST /widget/consultation, status_code=201, get_current_tenant auth, calls save_consultation() |
| `apps/efofx-estimate/app/services/widget_service.py` | save_consultation with email notification AND get_branding_by_prefix passing locale/consultation_form_labels | VERIFIED | save_consultation() at lines 113-137. BrandingConfigResponse constructor at lines 80-90 now includes locale=branding.locale (line 88) and consultation_form_labels=branding.consultation_form_labels (line 89). |
| `apps/efofx-estimate/app/models/widget.py` | ConsultationRequest Pydantic model | VERIFIED | Lines 86-93: ConsultationRequest with session_id, name, email, phone, message fields |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `apps/efofx-estimate/app/main.py` | `apps/efofx-estimate/app/db/mongodb.py` | lifespan calls migrate_estimation_session_tenant_id() | WIRED | Line 22 imports it; line 39 calls `await migrate_estimation_session_tenant_id()` |
| `apps/efofx-estimate/app/services/estimation_service.py` | `apps/efofx-estimate/app/models/estimation.py` | EstimationSession construction with tenant.tenant_id | WIRED | Line 179: `tenant_id=tenant.tenant_id` — correct UUID string, not random PyObjectId |
| `apps/efofx-widget/src/components/ConsultationCTA.tsx` | `apps/efofx-widget/src/components/ConsultationForm.tsx` | showForm state toggles rendering of ConsultationForm | WIRED | Line 15: `const [showForm, setShowForm] = useState(false)`. Lines 29-37: `if (showForm) { return <ConsultationForm ... /> }` |
| `apps/efofx-widget/src/components/ConsultationForm.tsx` | `apps/efofx-widget/src/api/chat.ts` | submitConsultation API call on form submit | WIRED | Line 3 imports `submitConsultation`; line 57: `await submitConsultation(apiKey, sessionId, formData)` in handleSubmit |
| `apps/efofx-estimate/app/api/widget.py` | `apps/efofx-estimate/app/services/widget_service.py` | POST /widget/consultation calls save_consultation() | WIRED | Line 33 imports `save_consultation`; lines 128-132: `lead_id = await save_consultation(tenant_id=..., consultation=..., contractor_email=...)` |
| `apps/efofx-estimate/app/services/widget_service.py` | `apps/efofx-estimate/app/models/widget.py` (BrandingConfigResponse) | get_branding_by_prefix returns locale/consultation_form_labels to frontend | WIRED | **Gap closed.** BrandingConfigResponse constructor at lines 80-90 now passes `locale=branding.locale` (line 88) and `consultation_form_labels=branding.consultation_form_labels` (line 89). Commit 8d58854. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DEBT-01 | 05-01-PLAN.md | Fix EstimationSession tenant_id type to match TenantAwareCollection expectations | SATISFIED | tenant_id: str in estimation.py; tenant.tenant_id in estimation_service.py; migration function in mongodb.py wired from lifespan |
| DEBT-02 | 05-01-PLAN.md | Add compound indexes for widget_analytics and widget_leads collections | SATISFIED | 3 indexes in create_indexes(): widget_analytics (tenant_id+date, unique), widget_leads (tenant_id+session_id), widget_leads (tenant_id+captured_at DESC) |
| DEBT-03 | 05-01-PLAN.md | Remove 5 deprecated collection accessors from mongodb.py | SATISFIED | All 5 functions absent from mongodb.py; zero imports across codebase; test_rcf_engine.py updated to use get_collection() |
| DEBT-04 | 05-02-PLAN.md + 05-03-PLAN.md | Wire ConsultationCTA button to contractor contact flow destination | SATISFIED | ConsultationCTA -> ConsultationForm -> POST /widget/consultation -> widget_leads fully wired. Per-tenant locale and form label overrides now propagate via branding API (gap closed in 05-03). |
| DEBT-05 | 05-01-PLAN.md | YAGNI pass — remove unused code paths and dead imports | SATISFIED | qa_epic2.py and test_count.py deleted; EstimationRequest, CostBreakdown, EstimationResult removed from estimation.py; unused imports cleaned up |
| DEBT-06 | 05-01-PLAN.md | Sync requirements.txt with pyproject.toml (fastapi-mail, valkey) | SATISFIED | All 5 missing entries added: fastapi-mail==1.6.2, valkey>=6.1.0, slowapi==0.1.9, pwdlib[bcrypt]==0.3.0, pydantic[email]>=2.11.0 |

All 6 requirement IDs (DEBT-01 through DEBT-06) are present in REQUIREMENTS.md and mapped to Phase 5 in the Traceability table. No orphaned requirements.

### Anti-Patterns Found

None. No TODO/FIXME markers, placeholder returns, or console stubs in any phase-modified files. The gap closure commit (8d58854) introduced no new anti-patterns.

### Human Verification Required

#### 1. Inline Form Rendering

**Test:** Open the widget in a browser, click "Request Free Consultation", observe the result.
**Expected:** Name, email, phone, and message input fields appear inline within the existing chat panel. The page does not navigate away, no modal overlay appears, and the form is visually contained within the widget.
**Why human:** CSS layout, panel containment, and responsive rendering cannot be verified programmatically.

#### 2. End-to-End Lead Persistence

**Test:** Submit the consultation form with test data and query the widget_leads MongoDB collection.
**Expected:** A document is present with fields: tenant_id (string UUID), session_id, name, email, phone, message, lead_type="consultation", captured_at (datetime).
**Why human:** Requires a live MongoDB instance with a seeded tenant and valid API key.

### Re-verification Summary

**Gap closed:** Truth #9 (locale/consultation_form_labels passthrough) is now VERIFIED. The one-line fix added `locale=branding.locale` and `consultation_form_labels=branding.consultation_form_labels` to the `BrandingConfigResponse` constructor in `get_branding_by_prefix()` at lines 88-89 of `apps/efofx-estimate/app/services/widget_service.py`. Commit `8d58854` confirmed in git history.

**Regressions:** None. Quick regression checks on all 8 previously passing truths confirm no changes to estimation.py, mongodb.py, main.py, requirements.txt, ConsultationCTA.tsx, ConsultationForm.tsx, or widget_service.py (other than the two-line fix).

**Automated verification complete.** The only remaining items are the two human verification tests above (visual form rendering and live database persistence), which were present in the initial verification and are unchanged.

---

_Verified: 2026-02-28_
_Verifier: Claude (gsd-verifier)_
