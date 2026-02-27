---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: unknown
last_updated: "2026-02-27T18:28:09.130Z"
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 17
  completed_plans: 16
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-26)

**Core value:** Trust through transparency — probabilistic estimates with explainable breakdowns that build contractor credibility with customers
**Current focus:** Phase 4 — White Label Widget (In Progress — 3 of 4 plans done)

## Current Position

Phase: 4 of 6 (White Label Widget) — In Progress
Plan: 3 of 4 in current phase (complete)
Status: Phase 4 Plan 03 complete — Chat UI, lead capture form, estimate display wired into widget. fetch+ReadableStream SSE, DOMPurify sanitization, P50/P80 range bar, accordion cost breakdown, streamed narrative. Requirements WFTR-01, WFTR-02, WFTR-03 satisfied.
Last activity: 2026-02-27 — Plan 04-03 complete: 6 API/hook files + 6 component files + ChatPanel orchestrator + App.tsx branding wiring + 350 lines CSS. Bundle builds to 637 kB (gzip 193 kB), zero TypeScript errors.

Progress: [█████░░░░░] 50%

## Performance Metrics

**Velocity:**
- Total plans completed: 13
- Average duration: ~4.5 min
- Total execution time: ~21 min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-prerequisites | 2 | 6 min | 3 min |
| 02-multi-tenant-foundation | 5 of 5 | 40 min | 8 min |
| 03-llm-integration | 4 of 4 | 18 min | 4.5 min |

**Recent Trend:**
- Last 5 plans: 5 min avg
- Trend: Fast

*Updated after each plan completion*
| Phase 02 P07 | 10 | 2 tasks | 3 files |
| Phase 03 P01 | 3 min | 2 tasks | 6 files |
| Phase 03 P02 | 3 min | 2 tasks | 7 files |
| Phase 03 P03 | 4 min | 2 tasks | 5 files |
| Phase 03 P04 | 8 min | 2 tasks | 5 files |
| Phase 04-white-label-widget P02 | 4 | 2 tasks | 7 files |
| Phase 04 P03 | 5 | 2 tasks | 15 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Pre-roadmap]: Phase 1 is prerequisites only — fix known bugs and replace abandoned packages before any feature work begins
- [Pre-roadmap]: Phases 1→2→3→4→5→6 are a hard dependency chain — no parallel execution of phases
- [Pre-roadmap]: Use PyJWT (not python-jose), pwdlib (not passlib), openai>=2.20.0, valkey (not redis)
- [Pre-roadmap]: TenantAwareCollection wrapper auto-injects tenant_id on all MongoDB operations — app-layer filtering is insufficient
- [Pre-roadmap]: Fernet encryption uses per-tenant HKDF-derived keys (not a shared master key) to limit blast radius
- [01-01]: Minimal query patch only for PRQT-02 — add $or clause to rcf_engine.py; Phase 2 builds TenantAwareCollection middleware for full coverage
- [01-01]: pytest-asyncio upgraded from 0.23.5 to 1.3.0 (incompatible with pytest 8.4.1); per-test Motor client pattern required for event-loop isolation in strict mode
- [01-02]: PyJWT 2.x encode() returns str directly — no .decode() needed; use jwt.InvalidTokenError not JWTError
- [01-02]: EstimationOutput uses P50/P80 per cost category with named AdjustmentFactor multipliers — matches transparency requirement
- [01-02]: generate_estimation() returns typed EstimationOutput (not Dict[str,Any]); callers needing dict use .model_dump()
- [01-02]: gpt-4o-mini required for client.beta.chat.completions.parse() structured outputs
- [01-02]: OpenAI structured output pattern established: always use beta.chat.completions.parse() with Pydantic response_format — never parse free-form LLM text
- [Phase 02]: TenantAwareCollection enforced at construction — ValueError on empty/None tenant_id makes mis-use impossible
- [Phase 02]: Per-operation collection instantiation (not stored in __init__) — avoids tenant_id drift across request lifecycle
- [Phase 02]: get_tenant_collection() is the single mandatory entry point for all tenant-scoped MongoDB access; deprecated get_*_collection() helpers retained only for tenants collection (non-tenant-scoped)
- [02-01]: Use BcryptHasher explicitly — PasswordHash.recommended() tries argon2 first but argon2 not installed; bcrypt is the Phase 1 dep
- [02-01]: api_key_last6 stored alongside hashed_api_key to enable masked display (sk-...abc123) without reversing bcrypt hash
- [02-01]: API key format sk_live_{tenant_id_no_dashes}_{random} enables O(1) tenant lookup by parsing first 32 chars — avoids full-collection bcrypt scan
- [Phase 02-multi-tenant-foundation]: SHA-256 (not bcrypt) for refresh token hashes — 384-bit entropy tokens are unguessable; SHA-256 enables O(1) MongoDB lookup
- [Phase 02-multi-tenant-foundation]: Refresh token rotation: old token deleted before new issued — any reuse of consumed token returns 401
- [Phase 02-multi-tenant-foundation]: AuthService class fully removed from security.py — standalone get_current_tenant supports JWT + API key dual auth
- [Phase 02-multi-tenant-foundation]: [02-02]: check_rate_limit removed from routes.py (dead code from removed RateLimiter); slowapi replaces in plan 02-05
- [02-04]: HKDF info string scoped to efofx-byok-{tenant_id} — per-tenant key derivation limits blast radius if master key is compromised
- [02-04]: openai_key_last6 stored alongside ciphertext for masked display without decryption
- [02-04]: PUT /auth/openai-key handles both initial store and rotation (simple overwrite, no version history per locked decision)
- [02-04]: 402 Payment Required is the gate for missing BYOK key — no platform key fallback
- [02-05]: slowapi headers_enabled=True incompatible with FastAPI Pydantic model returns; custom add_rate_limit_headers middleware reads request.state.view_rate_limit to inject X-RateLimit-* headers instead
- [02-05]: Rate limit test isolation: patch limiter._storage/_limiter to MemoryStorage on existing global instance (not new instance) — decorator captures global at import time
- [02-05]: Login rate limit key function falls back to ip:{addr} for unauthenticated requests — enables brute-force protection per IP
- [Phase 02]: BYOK-04 docs corrected: LLM endpoints return 402 when no BYOK key stored (no platform fallback) — aligns with locked decision from CONTEXT.md
- [Phase 02]: [02-07]: LLMService api_key fallback to settings.OPENAI_API_KEY retained ONLY for dev/testing; WILL be removed in Phase 3 (LLM-01) per-request injection plan
- [02-06]: get_all_tenant_statistics uses get_collection() (raw, unscoped) — intentional cross-tenant admin access, not a bug
- [02-06]: validate_tenant_limits derives monthly limit from TIER_LIMITS (trial=100, paid=1000) with settings override — Tenant model has no max_estimations_per_month field directly
- [02-06]: get_tenant() now returns dict (not Tenant model) using {"tenant_id": tenant_id} filter — consistent with get_by_tenant_id() contract
- [03-01]: LLMService.api_key is now required str with no default — settings.OPENAI_API_KEY fallback fully removed from production code paths
- [03-01]: In-memory _response_cache dict is per-process; upgrade to Valkey for multi-instance/multi-worker deployments
- [03-01]: classify_openai_error is module-level function (not method) for direct unit test import without LLMService instantiation
- [03-01]: Cache key uses sort_keys=True JSON + SHA-256 — ensures deterministic hashing regardless of dict insertion order
- [03-01]: use_cache=False parameter on generate_estimation enables forced refresh without cache invalidation complexity
- [03-02]: PromptService uses class-level _registry dict (not instance) — single shared registry loaded once at app startup, no DI complexity
- [03-02]: SHA-256 content hash stored at load time — immutability check compares hash on second load, not field-by-field diff
- [03-02]: prompt_version is Optional[str] = None on EstimationSession — backward-compatible with existing MongoDB documents
- [03-02]: Startup raises on PromptService failure — prompts are critical, fail fast rather than silently serve requests without registry
- [Phase 03]: ScopingContext extraction uses keyword/regex patterns (not LLM) — fast, cost-free, good enough for readiness detection
- [Phase 03]: Readiness requires all 4 fields: project_type, project_size, location, timeline — special_conditions is optional bonus
- [Phase 03]: Conversation preserved on ANY error — session persisted before returning error response, never raises from send_message
- [Phase 03]: Auto-trigger appends confirmation phrase to LLM response — not a separate message, feels natural in conversation
- [03-04]: EstimationSession.tenant_id uses PyObjectId() (fresh ObjectId) — legacy PyObjectId model incompatible with UUID tenant_id; TenantAwareCollection enforces tenant isolation at collection level
- [03-04]: Region enum fallback to NORCAL_BAY_AREA when ScopingContext.location doesn't match Region enum — keyword-extracted location strings may not match canonical enum values exactly
- [03-04]: SSE endpoint parameter must be named 'request' not 'http_request' — slowapi @limiter.limit inspects signature for exact name 'request' at decoration time
- [03-04]: Rate limiter disabled (limiter.enabled = False) in SSE tests — Valkey not available in unit test environment; same pattern as pre-existing constraint
- [04-01]: ?inline Vite import for widget.css is the only correct way to inject CSS into Shadow DOM — vite-plugin-css-injected-by-js injects to document.head which Shadow DOM does not inherit
- [04-01]: document.currentScript captured at module top level before any async operations — it becomes null after synchronous execution completes
- [04-01]: Host div for floating mode uses pointer-events:none; :host in CSS resets pointer-events:auto so shadow DOM buttons remain clickable
- [04-01]: all: initial on :host fully resets host-page style inheritance; explicit font-family reapplied after reset
- [Phase 04-white-label-widget]: Module-level _tenant_origins_cache dict shared by TenantAwareCORSMiddleware and widget_service — enables lazy CORS population without async DB calls in middleware
- [Phase 04-white-label-widget]: Branding endpoint uses key_func=get_remote_address explicitly — global limiter uses tenant-scoped key but public endpoint has no auth so IP-based limiting is required
- [Phase 04-white-label-widget]: Patch mocks at point of use (app.api.widget.save_lead) not at definition (app.services.widget_service.save_lead) — Python from-import creates local binding that must be patched where used
- [Phase 04]: fetch+ReadableStream for SSE (not EventSource) — EventSource cannot set Authorization headers; required for bearer-token-protected streams
- [Phase 04]: CSS custom properties applied from inside shadow root via getRootNode() instanceof ShadowRoot — avoids threading branding back up to ShadowDOMWrapper parent
- [Phase 04]: App.tsx owns branding fetch (not ChatPanel) — single source of truth passed to both WidgetProvider context and shadow root :host CSS override

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 4]: Verify react-shadow==20.6.0 compatibility with React 19.2.0 before widget work — fallback is manual useEffect + attachShadow()
- [Phase 4]: Verify Valkey SSL/TLS config (valkeys:// URI) works with valkey-py 6.1.0

## Session Continuity

Last session: 2026-02-27
Stopped at: Completed 04-03-PLAN.md — Chat UI, lead capture form, estimate display wired into widget. fetch+ReadableStream SSE, DOMPurify sanitization, P50/P80 range bar, accordion cost breakdown, streamed narrative. Requirements WFTR-01, WFTR-02, WFTR-03 satisfied.
Resume file: None
