# Phase 5: Tech Debt & Foundation Cleanup - Research

**Researched:** 2026-02-28
**Domain:** Codebase correctness, MongoDB indexing, data migration, React inline forms, dependency management
**Confidence:** HIGH — all findings are based on direct code inspection of the actual codebase

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**ConsultationCTA destination (DEBT-04)**
- Button opens an inline contact form within the widget (not a modal, not an external link)
- Form fields: name, email, phone number, free-text message
- Submission stores a document in widget_leads collection AND sends email notification to the contractor
- Form labels and placeholder text support locale-based defaults (en + es at launch) with per-tenant overrides in tenant config
- Per-tenant overrides take priority over locale defaults

**Dead code / YAGNI pass (DEBT-05)**
- Conservative scope: remove only clearly dead code — unused imports, unreachable functions, commented-out blocks
- Do NOT refactor working code or simplify over-engineered patterns — if it runs, leave it
- Covers both Python backend and React/JS frontend
- All commented-out code blocks are removed entirely (git history preserves them)
- Unused dependencies in pyproject.toml and package.json are removed

**Tenant ID migration (DEBT-01)**
- Fix the code to use the correct tenant identifier type going forward
- Write a migration to update existing EstimationSession documents to match the corrected type
- Migration runs automatically on deploy (application startup), not as a manual script
- Migration is idempotent — safe to re-run, no dry-run mode needed

**Compound indexes (DEBT-02)**
- widget_analytics and widget_leads indexes created via ensure_index on application startup
- Idempotent — always in sync with code, no separate migration script

### Claude's Discretion
- Deprecated accessor removal approach (DEBT-03) — straightforward deletion
- Dependency sync strategy (DEBT-06) — match requirements.txt to pyproject.toml
- Migration script structure and error handling
- Contact form validation rules and error messages
- Email notification template for contractor leads

### Deferred Ideas (OUT OF SCOPE)
- Full widget localization (all user-facing messaging across the entire widget, not just the contact form) — future phase
- Additional locale support beyond en + es — add as needed in future phases
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DEBT-01 | Fix EstimationSession tenant_id type to match TenantAwareCollection expectations (INT-04) | Section: DEBT-01 Findings — code location, type mismatch, migration pattern |
| DEBT-02 | Add compound indexes for widget_analytics and widget_leads collections (INT-05) | Section: DEBT-02 Findings — index design, startup hook location |
| DEBT-03 | Remove 5 deprecated collection accessors from mongodb.py | Section: DEBT-03 Findings — exact functions, caller map |
| DEBT-04 | Wire ConsultationCTA button to contractor contact flow destination | Section: DEBT-04 Findings — component location, existing form pattern, backend readiness |
| DEBT-05 | YAGNI pass — remove unused code paths and dead imports | Section: DEBT-05 Findings — known dead code inventory |
| DEBT-06 | Sync requirements.txt with pyproject.toml (fastapi-mail, valkey) | Section: DEBT-06 Findings — exact diffs |
</phase_requirements>

---

## Summary

Phase 5 is a correctness-and-cleanup phase with no new user-facing features except one: the ConsultationCTA contact form (DEBT-04). Every other item is a targeted fix or removal derived directly from the v1.0 milestone audit and the audit's INT-04/INT-05 findings. All items were verified by reading the actual source files.

The most complex work is DEBT-04: wiring the ConsultationCTA button to an inline contact form that submits to widget_leads AND emails the contractor. This is a small but real feature addition — it requires a new frontend component, a backend API endpoint, and email infrastructure. The email infrastructure must be careful: fastapi-mail is already declared in pyproject.toml (though absent from requirements.txt per DEBT-06) so no new dependencies are required. The form must support en/es locale defaults with per-tenant overrides from branding config.

The remaining items (DEBT-01 through DEBT-03, DEBT-05, DEBT-06) are mechanical: a one-line code fix plus idempotent startup migration, two index additions, five function deletions, targeted dead-code removal, and requirements.txt synchronization. None of these require library research. The correct implementation is dictated entirely by the existing codebase patterns already in place.

**Primary recommendation:** Implement in two plans matching the phase's existing plan split: Plan 05-01 covers the data-layer items (DEBT-01 migration, DEBT-02 indexes), Plan 05-02 covers the application-layer items (DEBT-03 accessor removal, DEBT-04 CTA form + email, DEBT-05 dead code, DEBT-06 dep sync).

---

## Standard Stack

No new dependencies are required for this phase. All libraries below are already declared in `pyproject.toml` and are in use.

### Core (already installed)

| Library | Version | Purpose | Usage in Phase |
|---------|---------|---------|----------------|
| motor | 3.3.2 | Async MongoDB driver | create_index() for DEBT-02, migration update_many for DEBT-01 |
| pymongo | 4.6.1 | MongoDB sync driver (used by motor) | Index key constants (ASCENDING = 1) |
| fastapi-mail | 1.6.2 | Email sending for contractor notifications | DEBT-04 email notification on lead submission |
| pydantic | 2.11.7 | Data models and validation | Contact form request/response models |
| React 19.2 | 19.2.0 | Widget frontend | New ContactForm component (DEBT-04) |
| TypeScript | ~5.9.3 | Widget type safety | ContactForm props and state types |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| valkey | >=6.1.0 | Declared in pyproject.toml, absent from requirements.txt | DEBT-06: add to requirements.txt only |
| pytest | 8.4.1 | Test runner | Verify migration, index creation |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| fastapi-mail | smtplib | fastapi-mail is already declared, uses same FastMail/MessageSchema pattern project already imports |
| Inline migration in startup | Alembic/migration script | Startup migration is simpler, idempotent, matches the create_indexes() pattern already in place |

**Installation:** No new packages required. DEBT-06 adds entries to requirements.txt only.

---

## Architecture Patterns

### Where Phase 5 Changes Live

```
apps/efofx-estimate/
├── app/
│   ├── db/
│   │   └── mongodb.py                  # DEBT-01 migration + DEBT-02 indexes + DEBT-03 deletions
│   ├── models/
│   │   └── estimation.py               # DEBT-01: change tenant_id type str
│   │   └── widget.py                   # DEBT-04: add ContactFormRequest model
│   ├── services/
│   │   └── widget_service.py           # DEBT-04: add save_consultation_request() + email send
│   ├── api/
│   │   └── widget.py                   # DEBT-04: add POST /widget/consultation endpoint
│   └── main.py                         # DEBT-01 migration called from lifespan
├── requirements.txt                    # DEBT-06: add fastapi-mail, valkey
apps/efofx-widget/src/
├── components/
│   ├── ConsultationCTA.tsx             # DEBT-04: replace console.log with form show/hide state
│   └── ConsultationForm.tsx            # DEBT-04: new inline contact form component
├── api/
│   └── chat.ts                         # DEBT-04: add submitConsultation() API call
└── types/
    └── widget.d.ts                     # DEBT-04: add ConsultationFormData type
```

### Pattern 1: Idempotent Startup Migration (DEBT-01)

**What:** A `migrate_*()` async function called from `lifespan()` in `main.py` after `connect_to_mongo()` and `create_indexes()`. Uses `update_many` with `$set` to convert incorrect values. Idempotent because `update_many` with a filter only touches documents that still need fixing.

**When to use:** Any one-time data correction that must run on every deploy but must be safe to re-run.

**Pattern (following existing startup convention):**
```python
# apps/efofx-estimate/app/db/mongodb.py

async def migrate_estimation_session_tenant_id():
    """
    DEBT-01: Convert EstimationSession.tenant_id from random PyObjectId to
    the correct tenant_id string stored by TenantAwareCollection.

    The bug: estimation_service.py line 180 set tenant_id=PyObjectId() (a random
    ObjectId) instead of tenant.tenant_id (a UUID string). TenantAwareCollection
    overwrites tenant_id on insert_one(), so the MongoDB document always has the
    correct string value. The EstimationSession.tenant_id field declaration used
    PyObjectId type, which is now being corrected to str.

    This migration: finds all documents in 'estimates' where tenant_id is stored
    as a BSON ObjectId (not a UUID string) and deletes or marks them as orphaned,
    since they were created with random ObjectIds and cannot be attributed to
    any real tenant. (TenantAwareCollection's insert_one overwrote them to the
    correct string before saving, so in practice no documents have ObjectId-type
    tenant_ids in MongoDB — this migration is a safety check.)

    Idempotent: update_many with filter is safe to run multiple times.
    """
    db = get_database()
    # In practice, documents always have string tenant_ids (TenantAwareCollection
    # overwrites before write). This confirms no BSON ObjectId-type values exist.
    result = await db["estimates"].count_documents(
        {"tenant_id": {"$type": "objectId"}}
    )
    if result > 0:
        logger.warning(
            "DEBT-01 migration: found %d estimates with ObjectId tenant_id — "
            "these cannot be attributed to a tenant and will be marked orphaned.",
            result,
        )
        await db["estimates"].update_many(
            {"tenant_id": {"$type": "objectId"}},
            {"$set": {"tenant_id": "__orphaned__", "migration_note": "DEBT-01: was random PyObjectId"}},
        )
    else:
        logger.info("DEBT-01 migration: no estimates with ObjectId tenant_id found — nothing to migrate")
```

**Then in `main.py` lifespan:**
```python
await connect_to_mongo()
await create_indexes()
await migrate_estimation_session_tenant_id()  # DEBT-01
```

### Pattern 2: create_indexes() Extension (DEBT-02)

**What:** Add two new index blocks to the existing `create_indexes()` function in `mongodb.py`. Follows the exact block-comment style already used for estimates, feedback, chat_sessions, etc.

**Example:**
```python
# ------------------------------------------------------------------
# Widget analytics — tenant_id first; daily bucketing
# ------------------------------------------------------------------
await db["widget_analytics"].create_index(
    [("tenant_id", 1), ("date", 1)],
    unique=True,
)
logger.info("Index confirmed: widget_analytics.tenant_id_1_date_1")

# ------------------------------------------------------------------
# Widget leads — tenant_id first; session lookup + time sort
# ------------------------------------------------------------------
await db["widget_leads"].create_index(
    [("tenant_id", 1), ("session_id", 1)]
)
logger.info("Index confirmed: widget_leads.tenant_id_1_session_id_1")
await db["widget_leads"].create_index(
    [("tenant_id", 1), ("captured_at", -1)]
)
logger.info("Index confirmed: widget_leads.tenant_id_1_captured_at_-1")
```

**Rationale:** `widget_analytics` uses `{tenant_id, date}` as the natural unique key (daily bucketing upsert in `record_analytics_event()`). `widget_leads` needs a session lookup index and a time-sort index (most recent leads per tenant).

### Pattern 3: Deprecated Accessor Removal (DEBT-03)

**What:** Delete 5 functions and their docstrings from `mongodb.py`. Update any callers to use `get_collection()` directly or `get_tenant_collection()` as appropriate. No replacement shim needed — all callers are test/script files, not production routes.

**The 5 functions to delete:**
1. `get_reference_classes_collection()` — DEPRECATED, callers: `qa_epic2.py`, `test_count.py`, `tests/services/test_rcf_engine.py`
2. `get_reference_projects_collection()` — DEPRECATED, no active callers in app code
3. `get_estimates_collection()` — DEPRECATED, no active callers in app code
4. `get_feedback_collection()` — DEPRECATED, no active callers in app code
5. `get_chat_sessions_collection()` — DEPRECATED, no active callers in app code

**Caller update strategy:**
- `apps/efofx-estimate/qa_epic2.py` — update to `get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])`
- `apps/efofx-estimate/test_count.py` — update to `get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])`
- `apps/efofx-estimate/tests/services/test_rcf_engine.py` — update to `get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])`

### Pattern 4: Inline Contact Form (DEBT-04)

**What:** Replace `ConsultationCTA.tsx`'s `console.info` handler with state that toggles a new `ConsultationForm` component inline in the chat panel. The form posts to a new backend endpoint `/api/v1/widget/consultation`.

**Frontend design:**
- `ConsultationCTA.tsx` gains `showForm` state. When `showForm=false`, renders the current disclaimer + button. When `showForm=true`, renders `<ConsultationForm>` in place of the button.
- `ConsultationForm.tsx` (new component) mirrors `LeadCaptureForm.tsx` structure: controlled inputs, validation, `isSubmitting` state, error display. Fields: name, email, phone, message (free text).
- Labels and placeholders come from a locale object. Default locale is `en`; `es` is bundled. The branding config (already available in widget context) may provide locale overrides via a `form_labels` key — if absent, use locale defaults.
- On successful submit: hide form, show a confirmation message in-place.

**Backend design:**
- New Pydantic model `ConsultationRequest` in `app/models/widget.py`: `session_id`, `name`, `email`, `phone`, `message`.
- New `save_consultation()` function in `widget_service.py`: saves to `widget_leads` collection (extends existing LeadCapture model with `message` field) AND sends email notification via `fastapi-mail`.
- New POST `/widget/consultation` endpoint in `app/api/widget.py`: auth-required, calls `save_consultation()`, returns 201.
- Email uses `fastapi-mail`'s `FastMail` and `MessageSchema` pattern (already declared in pyproject.toml).

**Locale pattern:**
```typescript
// apps/efofx-widget/src/i18n/consultationForm.ts  (new file)
export const CONSULTATION_FORM_LABELS = {
  en: {
    title: 'Request a Free Consultation',
    name: 'Your name',
    email: 'Email address',
    phone: 'Phone number',
    message: 'Tell us about your project',
    submit: 'Send Request',
    submitting: 'Sending...',
    success: 'Thank you! We\'ll be in touch soon.',
  },
  es: {
    title: 'Solicitar consulta gratuita',
    name: 'Tu nombre',
    email: 'Correo electrónico',
    phone: 'Número de teléfono',
    message: 'Cuéntanos sobre tu proyecto',
    submit: 'Enviar solicitud',
    submitting: 'Enviando...',
    success: '¡Gracias! Nos pondremos en contacto pronto.',
  },
} as const;

export type SupportedLocale = keyof typeof CONSULTATION_FORM_LABELS;
```

Per-tenant overrides: the `BrandingConfig` can include a `consultation_form_labels` key (partial overrides). These are merged over the locale defaults in the component.

### Pattern 5: Requirements.txt Sync (DEBT-06)

**What:** `requirements.txt` is missing `fastapi-mail==1.6.2` and `valkey>=6.1.0`. Also missing: `pydantic[email]`, `slowapi==0.1.9`, `pwdlib[bcrypt]==0.3.0`. Add them. The file also includes `gunicorn>=21.0.0`, `structlog>=24.0.0` which are not in pyproject.toml `[project].dependencies` — these are production-only; keep them.

**Exact additions needed:**
```
fastapi-mail==1.6.2
valkey>=6.1.0
slowapi==0.1.9
pwdlib[bcrypt]==0.3.0
pydantic[email]>=2.11.0
```

**Exact removals from requirements.txt (not in pyproject.toml, not production-needed):**
- `structlog>=24.0.0` — listed in pyproject.toml as optional? Actually NOT in pyproject.toml dependencies at all, but is in STACK.md. Keep it if it's actively imported; it is referenced in STACK.md but not in pyproject.toml. Keep for now — remove only if confirmed unused.
- `cryptography>=41.0.0` — also absent from pyproject.toml `[project].dependencies` but referenced in STACK.md as needed for BYOK. Keep.

**Strategy:** Add the 5 missing entries. Do not remove `gunicorn`, `structlog`, `cryptography` without confirming they're unused — that's beyond DEBT-06's scope (dep sync means making requirements.txt a superset of pyproject.toml, not trimming extras).

### Anti-Patterns to Avoid

- **Do NOT call `PromptService` or `LLMService` for the consultation form** — this is a simple storage + email operation, no LLM involvement.
- **Do NOT add `message` field to the existing `LeadCaptureRequest` model** — the consultation form is a separate flow from the pre-estimate lead capture. Create `ConsultationRequest` as a distinct model.
- **Do NOT use the TenantAwareCollection `aggregate()` method for the migration** — use raw `db["estimates"].update_many()` to avoid TenantAwareCollection's tenant_id scoping (the migration must touch ALL tenants' documents).
- **Do NOT add `tenant_id: PyObjectId` to any new model** — the EstimationSession model's `tenant_id` field should be typed as `str` going forward (matching what TenantAwareCollection injects).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Email sending | Custom SMTP client | `fastapi-mail` (already in pyproject.toml) | Already declared; has FastMail/MessageSchema pattern matching the project's async style |
| Locale string management | Complex i18n framework | Simple typed dict constant (see pattern above) | Only 2 locales at launch; full i18n framework is deferred |
| Idempotent index creation | Custom index existence check | Motor's `create_index()` built-in behavior | Motor's `create_index()` is idempotent by default — it will not fail if the index already exists |
| Form submission loading state | Complex state machine | Simple `isSubmitting: boolean` state (same as `LeadCaptureForm`) | Already proven pattern in the codebase |

---

## DEBT-01 Findings: EstimationSession tenant_id Type Mismatch

**The bug (INT-04 from milestone audit):**
- File: `apps/efofx-estimate/app/services/estimation_service.py`, line 180
- Code: `est_session = EstimationSession(tenant_id=PyObjectId(), ...)`
- `PyObjectId()` generates a **random** BSON ObjectId, not the tenant's actual UUID string
- `TenantAwareCollection.insert_one()` overwrites `tenant_id` with the correct string before the MongoDB write (line 132 of `tenant_collection.py`)
- **Result:** MongoDB document has the correct string tenant_id; the in-memory `est_session` object has a random ObjectId. Since `est_session` is returned by `generate_from_chat()` and immediately used by the SSE stream, and since no code reads `est_session.tenant_id` after construction, there is no user-visible bug. But the type is wrong.

**The fix (two parts):**
1. In `estimation.py`, change `EstimationSession.tenant_id: PyObjectId` to `tenant_id: str`
2. In `estimation_service.py` line 180, change `tenant_id=PyObjectId()` to `tenant_id=tenant.tenant_id`

**The migration (startup-time, idempotent):**
- In practice, all documents in `estimates` have string tenant_ids because TenantAwareCollection always overwrites. The migration is a safety assertion: count documents with BSON ObjectId-type `tenant_id`, log a warning if any exist, and update them to `"__orphaned__"` (since they cannot be attributed to a real tenant).
- This runs in `lifespan()` after `create_indexes()`.
- Pattern: new `async def migrate_estimation_session_tenant_id()` in `mongodb.py`, called from `main.py`.

**Confidence: HIGH** — verified by reading `estimation_service.py`, `tenant_collection.py`, `estimation.py`

---

## DEBT-02 Findings: Missing Widget Collection Indexes

**The gap (INT-05 from milestone audit):**
- `create_indexes()` in `mongodb.py` covers: tenants, estimates, reference_classes, reference_projects, feedback, chat_sessions, verification_tokens, refresh_tokens
- Missing: `widget_analytics`, `widget_leads`
- Both collections are actively used: `record_analytics_event()` does upserts on `widget_analytics`; `save_lead()` inserts into `widget_leads`
- Without indexes, `widget_analytics` queries in `get_analytics()` do full collection scans filtered by `tenant_id` and `date`

**Index design:**

`widget_analytics`:
- `(tenant_id: 1, date: 1)` — unique, covers the daily upsert in `record_analytics_event()` and the date-range query in `get_analytics()`
- The `get_analytics()` filter is `{"tenant_id": tenant.tenant_id, "date": {"$gte": start_date}}` — this composite index serves it directly

`widget_leads`:
- `(tenant_id: 1, session_id: 1)` — covers session-scoped lookups (if a consultation is opened after the initial lead)
- `(tenant_id: 1, captured_at: -1)` — covers time-sorted retrieval of leads per tenant

**Confidence: HIGH** — verified by reading `widget.py` (API), `widget_service.py`, and `mongodb.py`

---

## DEBT-03 Findings: Deprecated Collection Accessor Callers

**The 5 functions to delete from `mongodb.py`:**
```
get_reference_classes_collection()   — lines 128–133
get_reference_projects_collection()  — lines 136–141
get_estimates_collection()           — lines 144–149
get_feedback_collection()            — lines 152–157
get_chat_sessions_collection()       — lines 160–165
```

**All callers (verified by grep):**

| Caller File | Function Used | Fix |
|-------------|--------------|-----|
| `apps/efofx-estimate/qa_epic2.py` (line 8, 27) | `get_reference_classes_collection` | Replace with `get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])` |
| `apps/efofx-estimate/test_count.py` (line 2, 6) | `get_reference_classes_collection` | Replace with `get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])` |
| `apps/efofx-estimate/tests/services/test_rcf_engine.py` (line 37, 271) | `get_reference_classes_collection` | Replace with `get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])` |

No callers exist for `get_reference_projects_collection`, `get_estimates_collection`, `get_feedback_collection`, `get_chat_sessions_collection`.

**`get_tenants_collection()` — do NOT remove.** Its docstring explicitly notes it is the correct accessor for tenant management (top-level entities, not tenant-scoped). It is not deprecated.

**Confidence: HIGH** — verified by grep across entire workspace

---

## DEBT-04 Findings: ConsultationCTA Destination

**Current state:**
- `ConsultationCTA.tsx` has a button with `onClick={handleConsultationClick}` that does `console.info(...)` only
- The component has no state, no form, no API call
- `ChatPanel.tsx` renders `<ConsultationCTA />` after `<EstimateCard>` in the `result`/`generating` phases

**What needs to be built:**

*Frontend (React/TypeScript):*
1. `ConsultationCTA.tsx` — add `showForm: boolean` local state; on button click, set `showForm=true`. When `showForm=true`, render `<ConsultationForm>` instead of the button.
2. `ConsultationForm.tsx` (new) — mirrors `LeadCaptureForm.tsx` structure:
   - Props: `apiKey`, `sessionId`, `branding`, `onSubmitted`
   - Fields: name (text), email (email), phone (tel), message (textarea)
   - Locale labels from `CONSULTATION_FORM_LABELS[locale]` with per-tenant overrides merged on top
   - On submit: `POST /api/v1/widget/consultation`; on success: call `onSubmitted()` (which shows a thank-you message)
   - Locale detection: check `branding?.locale` (new optional field); default `'en'`
3. `i18n/consultationForm.ts` (new) — typed locale constants for `en` and `es`
4. `api/chat.ts` — add `submitConsultation(apiKey, sessionId, data)` function

*Backend (Python/FastAPI):*
1. `app/models/widget.py` — add `ConsultationRequest` model (session_id, name, email, phone, message)
2. `app/services/widget_service.py` — add `save_consultation(tenant_id, request, tenant_email)` that saves to `widget_leads` (with message field) and sends email via `fastapi-mail`
3. `app/api/widget.py` — add `POST /widget/consultation` endpoint (auth required, calls `save_consultation`, returns 201)
4. `app/models/widget.py` — extend `LeadCapture` to include optional `message: Optional[str]` field

**fastapi-mail usage pattern (for the email notification):**
```python
from fastapi_mail import FastMail, MessageSchema, ConnectionConfig

conf = ConnectionConfig(
    MAIL_USERNAME=settings.MAIL_USERNAME,
    MAIL_PASSWORD=settings.MAIL_PASSWORD,
    MAIL_FROM=settings.MAIL_FROM,
    MAIL_PORT=settings.MAIL_PORT,
    MAIL_SERVER=settings.MAIL_SERVER,
    MAIL_STARTTLS=True,
    MAIL_SSL_TLS=False,
)

async def send_consultation_email(to_email: str, lead: ConsultationRequest):
    message = MessageSchema(
        subject="New consultation request from your widget",
        recipients=[to_email],
        body=f"Name: {lead.name}\nEmail: {lead.email}\nPhone: {lead.phone}\n\n{lead.message}",
        subtype="plain",
    )
    fm = FastMail(conf)
    await fm.send_message(message)
```

**Settings additions needed (for email):**
- `MAIL_USERNAME`, `MAIL_PASSWORD`, `MAIL_FROM`, `MAIL_PORT`, `MAIL_SERVER` — add to `app/core/config.py` with sensible defaults (`Optional[str] = None`). Email sending should be gracefully skipped (log a warning) if not configured — don't fail the endpoint if email settings are missing.

**Contractor email address:** Retrieved from `Tenant.email` (already stored on the tenant document, used in auth flow). The `save_consultation()` service receives the authenticated `tenant` object from the route dependency.

**Confidence: HIGH** — verified by reading `ConsultationCTA.tsx`, `LeadCaptureForm.tsx`, `widget_service.py`, `widget.py` (API), `widget.d.ts`, `chat.ts`

---

## DEBT-05 Findings: Dead Code Inventory

Verified by reading the milestone audit, and code inspection. Conservative scope per locked decision.

**Python backend — confirmed dead code:**

| File | Dead item | Evidence |
|------|-----------|----------|
| `apps/efofx-estimate/qa_epic2.py` | Entire file — standalone QA script, no callers | Root-level script, not imported anywhere |
| `apps/efofx-estimate/test_count.py` | Entire file — standalone count script | Root-level script, not imported anywhere |
| `apps/efofx-estimate/app/models/estimation.py` | `EstimationRequest` class (lines 63-79) | Audit confirms "no active callers" — removed with INT-03 cleanup in Phase 4.1 |
| `apps/efofx-estimate/app/models/estimation.py` | `EstimationResult` class (lines 115-156) | Audit confirms "no active callers" — same reason |
| `apps/efofx-estimate/app/models/estimation.py` | `CostBreakdown` class (lines 82-112) | Used only by `EstimationResult` — if EstimationResult removed, CostBreakdown also dead |

**Note on `EstimationRequest` / `EstimationResult` / `CostBreakdown`:** These are in `estimation.py` alongside `EstimationSession`, `EstimationResponse`, `EstimationOutput` which are actively used. Verify no imports exist before deleting. The audit says no active callers (INT-03 endpoint was removed in Phase 4.1).

**Python backend — verify before removing:**
- `EstimationRequest` / `EstimationResult` / `CostBreakdown`: grep for imports across the codebase before deleting

**Frontend — confirmed dead code:**
- `ConsultationCTA.tsx` — the `console.info` handler body (the function stub, not the function itself — the function itself will be replaced by DEBT-04)
- No other frontend dead code confirmed without deeper inspection

**Unused imports:** Both Python and TypeScript linters should flag these. Run `flake8` on backend and `eslint` on frontend to catch unused imports — fix what's reported.

**Unused pyproject.toml / package.json dependencies:**
- `pyproject.toml`: all declared dependencies have confirmed usage
- `package.json` (widget): check with `depcheck` or manual review — defer if not immediately obvious

**Confidence: MEDIUM** — dead code items from audit are HIGH confidence; broader unused import sweep is MEDIUM (requires linter run to be exhaustive)

---

## DEBT-06 Findings: requirements.txt vs pyproject.toml Diff

**pyproject.toml `[project].dependencies` (source of truth):**
```
fastapi==0.116.1
uvicorn[standard]==0.27.1
motor==3.3.2
pymongo==4.6.1
pydantic==2.11.7
pydantic[email]>=2.11.0
pydantic-settings==2.2.1
openai>=2.20.0
PyJWT==2.11.0
pwdlib[bcrypt]==0.3.0
python-multipart==0.0.9
python-dotenv==1.0.1
python-dateutil==2.8.2
fastapi-mail==1.6.2          ← MISSING from requirements.txt
valkey>=6.1.0                ← MISSING from requirements.txt
slowapi==0.1.9               ← MISSING from requirements.txt
```

**In requirements.txt but NOT in pyproject.toml (keep — production/operational):**
```
gunicorn>=21.0.0             ← Production WSGI server — keep
structlog>=24.0.0            ← Structured logging — verify usage; keep if imported
cryptography>=41.0.0         ← BYOK Fernet encryption — keep
pytest==8.4.1                ← Also in pyproject.toml [dev] — keep in requirements.txt for CI
pytest-asyncio==1.3.0        ← Also in pyproject.toml [dev]
httpx==0.27.0                ← Also in pyproject.toml [dev]
black==24.1.1                ← Also in pyproject.toml [dev]
flake8==7.0.0                ← Also in pyproject.toml [dev]
mypy==1.8.0                  ← Also in pyproject.toml [dev]
pre-commit==3.6.2            ← Also in pyproject.toml [dev]
python-dateutil==2.8.2       ← In both — fine
```

**Also missing from requirements.txt (verify usage):**
- `pydantic[email]>=2.11.0` — used if `EmailStr` is referenced anywhere in models
- `pwdlib[bcrypt]==0.3.0` — used in `auth_service.py` for password hashing (replaces passlib)

**Action:** Add the 5 confirmed-missing entries to requirements.txt. Cross-check `pydantic[email]` and `pwdlib[bcrypt]` usage and add those too.

**Confidence: HIGH** — verified by direct comparison of both files

---

## Common Pitfalls

### Pitfall 1: Migration Touching TenantAwareCollection-Scoped Data

**What goes wrong:** If the migration uses `get_tenant_collection()` instead of raw `get_database()["estimates"]`, it will scope queries to a single tenant and miss all other tenants' documents.

**Why it happens:** The TenantAwareCollection wrapper prepends a `$match` on `tenant_id` to all queries. A migration that must touch ALL tenants cannot use this wrapper.

**How to avoid:** Always use `db = get_database(); db["estimates"].update_many(...)` — raw Motor collection — for cross-tenant migrations.

**Warning signs:** Migration reports 0 documents found but you know data exists.

---

### Pitfall 2: create_index() on widget_analytics with Wrong Uniqueness

**What goes wrong:** The `widget_analytics` daily bucketing uses `upsert=True` on `{"date": today}` within `TenantAwareCollection`. If the `(tenant_id, date)` index is NOT set to `unique=True`, MongoDB may create duplicate documents for the same tenant+date.

**Why it happens:** `upsert=True` with a non-unique filter can create duplicates if a race condition occurs. With a unique index, MongoDB enforces exactly-one-per-(tenant,date).

**How to avoid:** Add `unique=True` to the `widget_analytics` compound index. The existing `record_analytics_event()` code already relies on upsert semantics that assume uniqueness.

**Warning signs:** Multiple documents with the same `tenant_id` + `date` in `widget_analytics`.

---

### Pitfall 3: EstimationSession tenant_id Type Change Breaking Deserialization

**What goes wrong:** After changing `tenant_id: PyObjectId` to `tenant_id: str` in `EstimationSession`, existing MongoDB documents that happen to have an ObjectId-type `tenant_id` (if any exist from a buggy path) will fail Pydantic deserialization when `EstimationSession(**session_data)` is called.

**Why it happens:** Pydantic 2.x does not automatically coerce BSON ObjectId to str.

**How to avoid:** Run the migration BEFORE changing the Pydantic type in production. In practice, TenantAwareCollection overwrites tenant_id to a string before every MongoDB write, so no ObjectId-typed values should exist. The migration confirms this.

**Warning signs:** `ValidationError` when fetching existing estimation sessions after the deploy.

---

### Pitfall 4: Email Settings Not Configured on Startup

**What goes wrong:** If `MAIL_SERVER` and related settings are not in `.env`, `FastMail` initialization will fail or `send_message()` will throw, causing the consultation submission endpoint to return a 500.

**Why it happens:** The email settings are new. Existing deployments won't have them.

**How to avoid:** Make all `MAIL_*` settings `Optional[str] = None` in `config.py`. In `save_consultation()`, check if settings are configured before attempting to send email. If not configured, save the lead document and log a warning instead of raising.

**Warning signs:** 500 errors on `POST /widget/consultation` in any environment without email settings.

---

### Pitfall 5: Locale Override Merging Edge Cases

**What goes wrong:** If tenant config provides partial overrides (e.g., only `submit` label), and the merge logic is not defensive, the other labels may become `undefined`.

**Why it happens:** Naive spread `{ ...locale.en, ...tenantOverrides }` only works if `tenantOverrides` is a plain object. If it's `null` or `undefined` (branding not configured), it throws.

**How to avoid:**
```typescript
const labels = {
  ...CONSULTATION_FORM_LABELS[locale],
  ...(branding?.consultation_form_labels ?? {}),
};
```

**Warning signs:** Label fields rendering as empty strings or "undefined" in the form.

---

## Code Examples

### Example 1: Idempotent Index Creation (Motor Pattern)

```python
# Source: Motor documentation — create_index() is idempotent
await db["widget_analytics"].create_index(
    [("tenant_id", 1), ("date", 1)],
    unique=True,
)
# If index already exists with same spec, this is a no-op. No exception raised.
# If spec differs, raises OperationFailure — catch and log.
```

### Example 2: EstimationSession After Fix

```python
# Before (DEBT-01 bug):
est_session = EstimationSession(
    tenant_id=PyObjectId(),   # random ObjectId — WRONG
    ...
)

# After:
est_session = EstimationSession(
    tenant_id=tenant.tenant_id,   # correct UUID string from authenticated tenant
    ...
)
```

And in `estimation.py`:
```python
# Before:
class EstimationSession(BaseModel):
    tenant_id: PyObjectId = Field(..., description="Associated tenant ID")

# After:
class EstimationSession(BaseModel):
    tenant_id: str = Field(..., description="Associated tenant ID (UUID string)")
```

### Example 3: ConsultationForm TypeScript Shape

```typescript
// apps/efofx-widget/src/types/widget.d.ts — add:
export interface ConsultationFormData {
  name: string;
  email: string;
  phone: string;
  message: string;
}

// apps/efofx-widget/src/api/chat.ts — add:
export async function submitConsultation(
  apiKey: string,
  sessionId: string,
  data: ConsultationFormData
): Promise<void> {
  const res = await apiClient('/widget/consultation', apiKey, {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, ...data }),
  });
  if (!res.ok) throw new Error(`Consultation submit failed: ${res.status}`);
}
```

### Example 4: requirements.txt Additions (DEBT-06)

```
# Rate limiting
slowapi==0.1.9

# Email
fastapi-mail==1.6.2

# Cache
valkey>=6.1.0

# Auth (password hashing)
pwdlib[bcrypt]==0.3.0

# Pydantic email validation
pydantic[email]>=2.11.0
```

---

## State of the Art

| Old Approach | Current Approach | Impact for Phase 5 |
|--------------|------------------|--------------------|
| Separate migration scripts (`alembic`, `mongomigrate`) | Idempotent startup migration in `lifespan()` | DEBT-01: follow the startup pattern already established by `create_indexes()` |
| Motor `ensure_index()` (PyMongo 3.x) | Motor `create_index()` (PyMongo 4.x) | DEBT-02: use `create_index()` — it is idempotent and is the pattern already used in `create_indexes()` |
| Separate i18n library (i18next, react-intl) | Simple typed dict constant | DEBT-04: use the simple dict — full i18n is deferred |
| passlib | pwdlib[bcrypt] | Already migrated in the codebase — just a requirements.txt gap (DEBT-06) |

---

## Open Questions

1. **Does `BrandingConfig` already have a `locale` field or `consultation_form_labels` field?**
   - What we know: Current `BrandingConfig` in `widget.py` has no `locale` or `consultation_form_labels` fields.
   - What's unclear: Should these be added to `BrandingConfig` and the public branding API response? Or should locale be inferred from browser `navigator.language`?
   - Recommendation: Add optional `locale: str = "en"` and `consultation_form_labels: dict | None = None` to `BrandingConfig` model and `BrandingConfigResponse`. The widget reads `branding.locale` to pick the locale constant. This keeps per-tenant override capability in the existing branding config path.

2. **Should `qa_epic2.py` and `test_count.py` be deleted entirely (DEBT-05) or just fixed (DEBT-03)?**
   - What we know: Both are root-level standalone scripts, not part of the test suite or application code. They have no importers.
   - What's unclear: Were these for one-time use or ongoing QA? The audit calls them "test/script callers only" for the deprecated accessor.
   - Recommendation: Delete both entirely under DEBT-05 (dead code) rather than fixing under DEBT-03. They are clearly ad-hoc scripts and do not belong in the codebase. DEBT-03 then becomes simpler: only update `tests/services/test_rcf_engine.py`.

3. **What email provider is configured for production?**
   - What we know: `fastapi-mail` is declared but no `MAIL_*` settings exist in `app/core/config.py` or `.env.template`.
   - What's unclear: The email provider selection is deferred to Phase 7 (FEED-01). Should DEBT-04 just write to `widget_leads` and skip email entirely until Phase 7?
   - Recommendation: Implement email in DEBT-04 with graceful degradation — if `MAIL_SERVER` is not configured, skip email and log a warning. Phase 7 will set up the actual transactional email provider. The code infrastructure is ready but inactive until configured.

---

## Sources

### Primary (HIGH confidence)
- Direct code inspection: `apps/efofx-estimate/app/db/mongodb.py` — deprecated accessors, create_indexes(), TenantAwareCollection
- Direct code inspection: `apps/efofx-estimate/app/services/estimation_service.py` — bug at line 180
- Direct code inspection: `apps/efofx-estimate/app/models/estimation.py` — EstimationSession.tenant_id type
- Direct code inspection: `apps/efofx-widget/src/components/ConsultationCTA.tsx` — current console.log stub
- Direct code inspection: `apps/efofx-widget/src/components/LeadCaptureForm.tsx` — pattern to follow for DEBT-04
- Direct code inspection: `apps/efofx-estimate/app/api/widget.py` — existing widget endpoints
- Direct code inspection: `apps/efofx-estimate/app/services/widget_service.py` — save_lead() pattern
- Direct code inspection: `apps/efofx-estimate/pyproject.toml` — canonical dependencies
- Direct code inspection: `apps/efofx-estimate/requirements.txt` — missing entries
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md` — INT-04, INT-05, tech debt inventory
- `.planning/codebase/ARCHITECTURE.md`, `STRUCTURE.md`, `CONVENTIONS.md`, `TESTING.md` — project patterns
- grep: `get_estimates_collection|get_feedback_collection|...` — confirmed caller map for DEBT-03

### Secondary (MEDIUM confidence)
- `.planning/codebase/CONCERNS.md` — dead code inventory cross-reference
- `.planning/codebase/STACK.md` — confirms fastapi-mail, valkey are project dependencies

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — all libraries verified in pyproject.toml and in active use
- Architecture: HIGH — all patterns taken from existing working code in the codebase
- Pitfalls: HIGH for items 1-4 (derived from code analysis), MEDIUM for item 5 (locale merge — TypeScript runtime edge case)
- DEBT-01/02/03/06: HIGH — exact files, lines, and values confirmed by reading source
- DEBT-04: HIGH for structure, MEDIUM for email config details (provider not yet chosen)
- DEBT-05: HIGH for audit-listed items, MEDIUM for comprehensive unused-import sweep

**Research date:** 2026-02-28
**Valid until:** 2026-03-28 (stable codebase, 30 days reasonable)
