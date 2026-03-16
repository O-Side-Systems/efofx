# Shared Library Extraction Boundary Document

**Phase:** 09-shared-library-extraction
**Created:** 2026-03-16
**Purpose:** Resolve extraction decisions for every Python module and TypeScript component. This document is the single source of truth — subsequent plans execute these decisions without re-debating them.

---

## Python Modules — `apps/efofx-estimate/app/`

| Module | Location | Rationale |
|--------|----------|-----------|
| `app/utils/crypto.py` | EXTRACT → `packages/efofx-shared/efofx_shared/utils/crypto.py` | Pure stdlib + cryptography; zero app imports; already tested in `tests/utils/test_crypto.py`; no FastAPI/Motor in transitive dep graph |
| `app/core/constants.py` (enums only: `EstimationStatus`, `ReferenceClassCategory`, `CostBreakdownCategory`, `Region`) | EXTRACT → `packages/efofx-shared/efofx_shared/core/constants.py` | Pure Python enums; no app imports; reusable across verticals |
| `app/core/constants.py` (non-enum values: `API_MESSAGES`, `ESTIMATION_CONFIG`, `LLM_PROMPTS`, `DB_COLLECTIONS`, `HTTP_STATUS`, `FILE_UPLOAD_CONFIG`) | STAYS in `apps/efofx-estimate` | Estimation-domain constants; not reusable across verticals |
| `app/utils/calculation_utils.py` | STAYS in `apps/efofx-estimate` | Pool construction domain logic (region multipliers, labor costs, pool sizes); hardcoded estimation-specific values; misleading to future IT/dev vertical if moved to shared |
| `app/utils/validation_utils.py` | STAYS in `apps/efofx-estimate` | Imports `app.core.constants` (Region, ReferenceClassCategory); validation logic is estimation-specific (pool size ranges, square footage); not reusable across verticals |
| `app/utils/file_utils.py` | STAYS in `apps/efofx-estimate` | Imports `fastapi.UploadFile`; cannot extract without removing FastAPI dependency; extraction would require architectural change (Rule 4) |
| `app/models/_objectid.py` | STAYS in `apps/efofx-estimate` | MongoDB ObjectId serializer; imports `bson.ObjectId` (pymongo/motor ecosystem); database-specific code that must not pollute shared package |
| `app/core/config.py` | STAYS in `apps/efofx-estimate` | App settings (database URL, API keys, SMTP config); entirely app-specific; shared code must receive config as parameters, not read from `app.core.config` |
| `app/core/security.py` | STAYS in `apps/efofx-estimate` | JWT auth + tenant verification; imports FastAPI `Depends`; tightly coupled to API layer |
| `app/core/rate_limit.py` | STAYS in `apps/efofx-estimate` | slowapi rate limiting; FastAPI middleware; not reusable outside the API app |
| `app/services/*` | STAYS in `apps/efofx-estimate` | All business logic services (CalibrationService, FeedbackService, etc.); coupled to Motor DB, tenant system, FastAPI |
| `app/api/*` | STAYS in `apps/efofx-estimate` | All FastAPI routers and endpoint handlers; inherently app-specific |
| `app/db/*` | STAYS in `apps/efofx-estimate` | Database connection, TenantAwareCollection; Motor/MongoDB specific |
| `app/models/*` (except `_objectid.py`) | STAYS in `apps/efofx-estimate` | Pydantic models reference estimation-domain schemas (EstimationSession, CalibrationResult, etc.) |

---

## TypeScript Components — `apps/efofx-widget/` and `apps/efofx-dashboard/`

| Component | Location | Rationale |
|-----------|----------|-----------|
| `ChatBubble.tsx` | EXTRACT → `packages/efofx-ui/src/components/ChatBubble/` | Pure presentational; receives `ChatMessage` type (role + content string); no hooks; no widget-specific state |
| `EstimateCard.tsx` | EXTRACT → `packages/efofx-ui/src/components/EstimateCard/` | Uses local `useState` only; define standalone `EstimationOutput` interface inside efofx-ui to avoid cross-package type dependency on widget app |
| `TypingIndicator.tsx` | EXTRACT → `packages/efofx-ui/src/components/TypingIndicator/` | Zero props, zero imports; purest extraction candidate |
| `ErrorBoundary.tsx` | EXTRACT → `packages/efofx-ui/src/components/ErrorBoundary/` | Generic React error boundary; no estimation domain logic; reusable across any React app |
| `LoadingSkeleton.tsx` (dashboard) | EXTRACT → `packages/efofx-ui/src/components/LoadingSkeleton/` | Generic skeleton UI; no calibration-specific logic; parameterized by width/height only |
| `ChatPanel.tsx` | STAYS in `apps/efofx-widget` | Uses `useChat` hook and widget-specific state; tightly coupled to widget embed mechanism |
| `FloatingButton.tsx` | STAYS in `apps/efofx-widget` | Widget-specific UI element (expand/collapse chat); not reusable |
| `ConsultationCTA.tsx` | STAYS in `apps/efofx-widget` | Estimation domain; links to ConsultationForm; references estimation workflow |
| `ConsultationForm.tsx` | STAYS in `apps/efofx-widget` | Estimation domain; uses `useChat` hook; contacts backend estimation API |
| `LeadCaptureForm.tsx` | STAYS in `apps/efofx-widget` | Estimation domain; lead capture for pool estimation leads |
| `NarrativeStream.tsx` | STAYS in `apps/efofx-widget` | SSE streaming logic; widget-specific; not a reusable UI primitive |
| `ShadowDOMWrapper.tsx` | STAYS in `apps/efofx-widget` | Widget-embed mechanism (Shadow DOM); not a reusable UI component |
| `AccuracyBucketBar.tsx` | STAYS in `apps/efofx-dashboard` | Calibration domain (accuracy bucket visualization); depends on estimation-specific data shape |
| `AccuracyTrendLine.tsx` | STAYS in `apps/efofx-dashboard` | Calibration trend; uses Recharts + self-fetching hook; calibration-domain specific |
| `CalibrationMetrics.tsx` | STAYS in `apps/efofx-dashboard` | Calibration domain; displays estimation accuracy metrics |
| `DateRangeFilter.tsx` | STAYS in `apps/efofx-dashboard` | Calibration dashboard UI; estimation-specific filter for calibration time ranges |
| `ReferenceClassTable.tsx` | STAYS in `apps/efofx-dashboard` | Calibration domain; reference class accuracy breakdown |
| `ThresholdProgress.tsx` | STAYS in `apps/efofx-dashboard` | Calibration threshold progress; estimation-domain concept (10-outcome minimum) |

---

## Extraction Rules (for future phases)

1. **No app.* imports in efofx-shared** — shared code receives config as parameters, never reads from `app.core.config`
2. **No FastAPI/Motor/uvicorn in efofx-shared pyproject.toml** — these are app-server dependencies, not utility dependencies
3. **No estimation-domain types in @efofx/ui** — `EstimationOutput` is redefined as a local interface inside the UI package
4. **CSS modules use camelCase** — global names like `efofx-bubble-wrapper` become `bubbleWrapper` in CSS modules
5. **Workspace protocol only** — no PyPI or npm publishing; `{ workspace = true }` for Python, `workspace:*` for TypeScript
