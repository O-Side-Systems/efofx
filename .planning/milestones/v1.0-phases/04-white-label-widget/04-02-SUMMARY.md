---
phase: 04-white-label-widget
plan: 02
subsystem: api
tags: [fastapi, pydantic, cors, mongodb, slowapi, widget, branding]

# Dependency graph
requires:
  - phase: 02-multi-tenant-foundation
    provides: TenantAwareCollection, get_tenant_collection, Tenant model, get_current_tenant dependency, TrustedHostMiddleware, slowapi limiter
  - phase: 03-llm-integration
    provides: DB connection patterns, test isolation patterns, rate limiter disable pattern in tests

provides:
  - "Public GET /api/v1/widget/branding/{api_key_prefix} endpoint (no auth, 30/min rate limit)"
  - "BrandingConfig, BrandingConfigResponse, LeadCapture, LeadCaptureRequest, LeadCaptureResponse, WidgetAnalyticsEvent Pydantic models"
  - "widget_service: get_branding_by_prefix, save_lead, record_analytics_event, get_tenant_allowed_origins"
  - "TenantAwareCORSMiddleware with module-level _tenant_origins_cache for lazy per-tenant CORS"
  - "POST /api/v1/widget/lead (API key auth, 201)"
  - "POST /api/v1/widget/analytics (API key auth, 204)"
  - "DB_COLLECTIONS WIDGET_LEADS and WIDGET_ANALYTICS constants"
  - "11 tests passing in tests/api/test_widget.py"

affects:
  - 04-03-widget-frontend
  - 04-04-widget-embed

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Module-level cache dict (_tenant_origins_cache) shared between middleware and service layer for lazy CORS population"
    - "Public endpoint with @limiter.limit and key_func=get_remote_address for IP-based rate limiting without auth"
    - "TenantAwareCORSMiddleware.is_allowed_origin checks static origins first (fast path), then per-tenant cache"
    - "Patch import path at point of use (app.api.widget.save_lead) not at definition (app.services.widget_service.save_lead)"

key-files:
  created:
    - apps/efofx-estimate/app/models/widget.py
    - apps/efofx-estimate/app/services/widget_service.py
    - apps/efofx-estimate/app/api/widget.py
    - apps/efofx-estimate/app/middleware/cors.py
    - apps/efofx-estimate/tests/api/test_widget.py
  modified:
    - apps/efofx-estimate/app/main.py
    - apps/efofx-estimate/app/core/constants.py

key-decisions:
  - "Module-level _tenant_origins_cache dict (not class attribute) shared by middleware and widget_service — enables cache updates without ASGI middleware reference"
  - "CORS origins populated lazily on first branding request per tenant — avoids loading all tenant origins at startup"
  - "Branding endpoint uses key_func=get_remote_address in @limiter.limit decorator — IP-based limit (not tenant) because endpoint is public/unauthenticated"
  - "Patch mock at app.api.widget.save_lead (point of use) not app.services.widget_service.save_lead — Python import binding requires patching where name is used"
  - "AnalyticsRequest defined inline in widget.py (not in models) — avoids circular import and keeps the one-field model close to its single endpoint"
  - "record_analytics_event uses try/except to swallow errors — fire-and-forget semantics must not propagate analytics failures to widget users"

patterns-established:
  - "Module-level cache: use a module-level dict for shared state between middleware and service layers when async DB calls in middleware are impractical"
  - "Public endpoint rate limiting: use key_func=get_remote_address explicitly in @limiter.limit when the global limiter uses a different key function"

requirements-completed: [BRND-01, BRND-02, BRND-03, BRND-04, WDGT-05]

# Metrics
duration: 4min
completed: 2026-02-27
---

# Phase 4 Plan 2: Widget Backend Branding API and CORS Summary

**Public GET /branding/{prefix} endpoint (30/min rate-limited, no auth), per-tenant TenantAwareCORSMiddleware with lazy origin cache, and 11 widget tests passing**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-02-27T18:13:42Z
- **Completed:** 2026-02-27T18:17:10Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Public branding endpoint returns all 7 branding fields (primary_color, secondary_color, accent_color, logo_url, welcome_message, button_text, company_name) without requiring auth, rate-limited 30/min per IP
- TenantAwareCORSMiddleware extends Starlette's CORSMiddleware with per-tenant dynamic origin checking via lazily-populated module-level cache — avoids async DB calls in synchronous middleware
- 11 tests passing covering branding endpoint (200, 404, defaults, custom values, no sensitive fields, no auth required, company_name), lead capture (401 without auth, 201 with mock DB), and CORS (static origins, tenant-registered origins)

## Task Commits

Each task was committed atomically:

1. **Task 1: Widget models, service, and branding API endpoint** - `10239df` (feat)
2. **Task 2: Per-tenant CORS middleware and widget endpoint tests** - `a06612c` (feat)

**Plan metadata:** (included in final docs commit)

## Files Created/Modified

- `apps/efofx-estimate/app/models/widget.py` - BrandingConfig, BrandingConfigResponse, LeadCapture, LeadCaptureRequest, LeadCaptureResponse, WidgetAnalyticsEvent models
- `apps/efofx-estimate/app/services/widget_service.py` - get_branding_by_prefix (prefix-to-UUID, branding merge, CORS cache update), save_lead, record_analytics_event, get_tenant_allowed_origins
- `apps/efofx-estimate/app/api/widget.py` - widget_router: GET /widget/branding/{prefix}, POST /widget/lead, POST /widget/analytics
- `apps/efofx-estimate/app/middleware/cors.py` - TenantAwareCORSMiddleware with _tenant_origins_cache module-level dict
- `apps/efofx-estimate/tests/api/test_widget.py` - 11 tests for branding endpoint, lead capture auth, and CORS behavior
- `apps/efofx-estimate/app/main.py` - Replace CORSMiddleware with TenantAwareCORSMiddleware, add widget_router import and include_router
- `apps/efofx-estimate/app/core/constants.py` - Add WIDGET_LEADS and WIDGET_ANALYTICS to DB_COLLECTIONS

## Decisions Made

- **Module-level cache:** Used a module-level `_tenant_origins_cache: dict[str, list[str]]` dict in `cors.py` (not a class attribute or app.state reference) so both the middleware and widget_service can import it directly. Starlette's `add_middleware` creates its own middleware instance, making app.state references from middleware complex.
- **Lazy CORS population:** Origins are cached on first branding request per tenant, not at startup. This avoids loading all tenants' origin lists at startup and avoids requiring async DB calls inside synchronous middleware.
- **IP-based rate limit on public endpoint:** `@limiter.limit("30/minute", key_func=get_remote_address)` — the global limiter uses `get_tenant_id_for_limit` (tenant-scoped), but the branding endpoint has no auth so IP-based limiting is the correct approach.
- **Patch at point of use:** Test for `save_lead` patches `app.api.widget.save_lead` (not `app.services.widget_service.save_lead`) because Python's `from module import name` creates a local binding in the importing module.

## Deviations from Plan

None - plan executed exactly as written.

**One minor fix during testing (not a deviation):** Patching `app.api.widget.save_lead` instead of `app.services.widget_service.save_lead` — this is the correct Python mock pattern when using `from X import Y`, not a plan deviation.

## Issues Encountered

None — all 11 tests passed after correcting the mock patch path.

## User Setup Required

None - no external service configuration required. Widget backend endpoints are ready for consumption by Plans 04-03 and 04-04.

## Next Phase Readiness

- Public branding API ready: GET /api/v1/widget/branding/{32-char-prefix} → BrandingConfigResponse (Plans 04-03/04-04 can fetch branding before rendering)
- Lead capture API ready: POST /api/v1/widget/lead with API key → 201 Created
- Analytics event API ready: POST /api/v1/widget/analytics with API key → 204 No Content
- CORS middleware ready: tenant-registered domains allowed via lazy cache populated during branding fetch
- All 5 requirements satisfied: BRND-01 through BRND-04, WDGT-05

---
*Phase: 04-white-label-widget*
*Completed: 2026-02-27*

## Self-Check: PASSED
