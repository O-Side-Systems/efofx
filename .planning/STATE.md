---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: in_progress
last_updated: "2026-02-27T17:29:00.000Z"
progress:
  total_phases: 6
  completed_phases: 2
  total_plans: 13
  completed_plans: 10
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-26)

**Core value:** Trust through transparency — probabilistic estimates with explainable breakdowns that build contractor credibility with customers
**Current focus:** Phase 3 — LLM Integration (in progress, plan 1 of 4 complete)

## Current Position

Phase: 3 of 6 (LLM Integration) — IN PROGRESS
Plan: 1 of 4 in current phase (complete)
Status: Phase 3 Plan 01 complete — LLMService hardened: BYOK-only, error classification, SHA-256 caching, get_llm_service DI, EstimationService/ChatService rewired to accept LLMService via constructor injection; 16 new tests passing
Last activity: 2026-02-27 — Plan 03-01 complete: settings fallback removed, classify_openai_error added, _make_cache_key + _response_cache added, stream_chat_completion added, get_llm_service FastAPI dependency added, routes/services rewired for DI

Progress: [████████░░] 62%

## Performance Metrics

**Velocity:**
- Total plans completed: 2
- Average duration: 3 min
- Total execution time: 6 min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-prerequisites | 2 | 6 min | 3 min |
| 02-multi-tenant-foundation | 5 of 5 | 40 min | 8 min |
| 03-llm-integration | 1 of 4 | 3 min | 3 min |

**Recent Trend:**
- Last 5 plans: 6 min avg
- Trend: Fast

*Updated after each plan completion*
| Phase 02 P07 | 10 | 2 tasks | 3 files |
| Phase 03 P01 | 3 min | 2 tasks | 6 files |

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

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 4]: Verify react-shadow==20.6.0 compatibility with React 19.2.0 before widget work — fallback is manual useEffect + attachShadow()
- [Phase 4]: Verify Valkey SSL/TLS config (valkeys:// URI) works with valkey-py 6.1.0

## Session Continuity

Last session: 2026-02-27
Stopped at: Completed 03-01-PLAN.md — LLMService hardened: BYOK-only constructor, classify_openai_error, SHA-256 caching, get_llm_service DI dependency, stream_chat_completion. EstimationService and ChatService rewired to accept LLMService via constructor. Routes updated. 16 tests passing. Requirements LLM-01, LLM-03, LLM-04 satisfied.
Resume file: None
