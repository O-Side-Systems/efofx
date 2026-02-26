# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-26)

**Core value:** Trust through transparency — probabilistic estimates with explainable breakdowns that build contractor credibility with customers
**Current focus:** Phase 1 — Prerequisites (complete)

## Current Position

Phase: 1 of 6 (Prerequisites)
Plan: 2 of 2 in current phase (phase complete)
Status: Phase 1 complete — ready for Phase 2
Last activity: 2026-02-26 — Plan 01-02 complete: dependency migration (PyJWT, pwdlib, openai v2, structured outputs)

Progress: [██░░░░░░░░] 17%

## Performance Metrics

**Velocity:**
- Total plans completed: 2
- Average duration: 3 min
- Total execution time: 6 min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-prerequisites | 2 | 6 min | 3 min |

**Recent Trend:**
- Last 5 plans: 3 min avg
- Trend: Fast

*Updated after each plan completion*

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

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 4]: Verify react-shadow==20.6.0 compatibility with React 19.2.0 before widget work — fallback is manual useEffect + attachShadow()
- [Phase 4]: Verify Valkey SSL/TLS config (valkeys:// URI) works with valkey-py 6.1.0

## Session Continuity

Last session: 2026-02-26
Stopped at: Completed 01-02-PLAN.md — Phase 1 complete, ready for Phase 2 (tenant isolation)
Resume file: None
