# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-02-28
**Phases:** 5 | **Plans:** 18 | **Commits:** 77

### What Was Built
- Multi-tenant SaaS foundation: registration, JWT auth, TenantAwareCollection isolation, BYOK encryption, rate limiting
- Real OpenAI LLM integration: BYOK per-request injection, versioned prompts, response caching, SSE streaming
- Conversational scoping engine: multi-turn chat, follow-up questions, auto-trigger estimation
- White-label embeddable widget: Shadow DOM isolation, branding, chat UI, lead capture, estimate display
- Integration gap closure: tenant accessor fixes, startup index creation, dead code removal

### What Worked
- **Hard dependency chain** (1→2→3→4) prevented integration surprises — each phase built cleanly on the previous
- **TenantAwareCollection pattern** eliminated entire categories of cross-tenant bugs — enforcement at construction time
- **2-task-per-plan structure** kept plans atomic and fast (~5 min avg execution)
- **Milestone audit before completion** caught 3 real integration issues (INT-01/02/03) that would have shipped broken
- **Phase 4.1 decimal insertion** cleanly addressed audit gaps without disrupting roadmap numbering

### What Was Inefficient
- **SUMMARY frontmatter gaps** — Phase 3 summaries all missing `requirements-completed` field; documentation discipline dropped during fast execution
- **Per-process LLM cache** — simplest approach but known limitation from day 1; should have added Valkey backing from the start
- **EstimationSession tenant_id type mismatch** (INT-04) — PyObjectId() used instead of tenant.tenant_id; TenantAwareCollection masked the bug by overwriting on insert
- **Widget collection indexes omitted** (INT-05) — create_indexes() didn't cover analytics/leads collections added in Phase 4

### Patterns Established
- **TenantAwareCollection** as the single entry point for all tenant-scoped MongoDB operations
- **OpenAI structured output** via `beta.chat.completions.parse()` with Pydantic response_format — never parse free-form LLM text
- **Shadow DOM CSS injection** via Vite `?inline` import — the only correct approach for style isolation
- **fetch+ReadableStream for SSE** (not EventSource) — required for bearer-token-protected streams
- **slowapi + custom header middleware** — slowapi's headers_enabled incompatible with Pydantic model responses
- **Per-operation collection instantiation** — avoids tenant_id drift across request lifecycle
- **Patch mocks at point of use** — Python from-import creates local binding

### Key Lessons
1. **Milestone audits are essential** — they caught real bugs (tenant.id AttributeError) that unit tests missed because tests used different code paths
2. **Decimal phases work well for urgent insertions** — Phase 4.1 was clean to plan, execute, and reference without renumbering
3. **TenantAwareCollection-style enforcement patterns** prevent entire bug categories — enforce at construction, not at query time
4. **Documentation discipline drops under velocity** — frontmatter fields are easy to skip when executing fast; consider automation
5. **Shadow DOM requires specific CSS/JS patterns** — every standard web pattern (EventSource, CSS inheritance, document.currentScript) has a Shadow DOM-specific workaround

### Cost Observations
- Model mix: balanced profile (sonnet for research/planning agents, opus for execution)
- Sessions: ~8 sessions across 2 days
- Notable: 18 plans in 2 days — high velocity sustained by yolo mode + 2-task atomic plans

---

## Milestone: v1.1 — Feedback & Quality

**Shipped:** 2026-03-16
**Phases:** 5 | **Plans:** 15 | **Commits:** 69

### What Was Built
- Fixed all v1.0 audit tech debt: tenant_id type, widget indexes, deprecated accessors, dead code, ConsultationCTA with inline form and i18n
- Distributed Valkey cache replacing per-process LLM dict with tenant-scoped keys and graceful fallback
- Feedback email system: Resend SDK, magic link tokens (SHA-256, 72h TTL, two-step validation), immutable estimate snapshots
- Calibration dashboard: CalibrationService with tenant-scoped $lookup aggregation, 10-outcome threshold, React 19 + Recharts SPA
- Shared library extraction: packages/efofx-shared (Python), packages/efofx-ui (React), uv + npm workspaces, CI isolation tests, STANDARDS.md

### What Worked
- **Wave-based parallelism** — Phases 7 and 8 both had Wave 1 (parallel foundation) + Wave 2 (depends on both); reduced critical path
- **Gap closure pattern from v1.0 carried forward** — Phase 5 was entirely gap closure; 05-03 closed a gap found by 05-VERIFICATION
- **Re-export shim pattern** — extracting to shared packages with `from efofx_shared.x import *` shims meant zero caller changes across 18+ files
- **Two-step token validation** (GET idempotent, POST consumes) — prevented email security scanners from consuming magic links; elegant design
- **CSS custom properties over Tailwind** — dashboard achieved clean Stripe/Linear aesthetic with ~100 lines of CSS variables
- **Quick task for deployment config** — used /gsd:quick to close audit tech debt (4 env vars in app.yaml) without full phase ceremony

### What Was Inefficient
- **SUMMARY frontmatter recording gaps** — 8 of 27 requirements not listed in `requirements_completed` YAML across plans 05-01, 07-02, 09-01; same documentation discipline issue from v1.0
- **Phase 8 roadmap checkbox not updated** — `[ ]` instead of `[x]` despite all 3 plans being complete; roadmap discipline dropped
- **uv workspace config location** — initially put workspace config in uv.toml (wrong); had to move to pyproject.toml [tool.uv.workspace]; research should have caught this
- **Azure DevOps pip index conflict** — blocked uv sync in dev environment; required `uv pip install -e` workaround; dev environment setup docs needed

### Patterns Established
- **ValkeyCache singleton with lazy init** — `_get_client()` avoids connection at import time; warning cooldown via module-level `_last_warn_at`
- **Jinja2 Environment loaded once at module level** — `_jinja_env` singleton avoids filesystem I/O per request
- **BackgroundTasks for fire-and-forget email** — async closure wrapping email dispatch, non-blocking
- **MongoDB TTL index for token expiry** — `expireAfterSeconds=0` on expires_at field, automatic cleanup
- **Re-export shim for shared package extraction** — `from efofx_shared.utils.crypto import *` in app file, zero caller disruption
- **Source imports for shared TypeScript** — `main: ./src/index.ts` in package.json, consuming Vite resolves TypeScript directly (no build step)
- **AccuracyTrendLine self-fetches via useCalibrationTrend** — component-level data fetching reduces prop drilling
- **CalibrationService $lookup with explicit tenant_id in inner pipeline** — TenantAwareCollection only scopes source collection

### Key Lessons
1. **SUMMARY frontmatter discipline still needs automation** — same gap as v1.0; consider enforcing `requirements_completed` as a required field in executor
2. **Deployment config is a first-class artifact** — 4 env vars missing from app.yaml nearly broke production; app.yaml should be part of every phase that introduces a new env var
3. **Two-step token validation is the right pattern for email links** — GET idempotent (safe for scanners), POST consumes; this should be reused for any future email-linked action
4. **Workspace tooling (uv, npm) has sharp edges** — workspace config location, private pip index conflicts, npm vs pnpm dependency syntax all required real debugging; research phases should explicitly verify workspace tooling behavior
5. **Quick tasks fill the gap between "no phase needed" and "just do it"** — audit tech debt items are a perfect quick task use case

### Cost Observations
- Model mix: balanced profile (sonnet for agents, opus for orchestration)
- Sessions: ~12 sessions across 16 days
- Notable: 15 plans in 16 days — steady pace vs v1.0's sprint; feature complexity higher (email, dashboard, shared packages)

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 77 | 5 | Established GSD workflow with milestone audit → gap closure pattern |
| v1.1 | 69 | 5 | Added quick tasks for small fixes, wave-based parallel planning matured |

### Cumulative Quality

| Milestone | Requirements | Satisfied | Audit Status |
|-----------|-------------|-----------|--------------|
| v1.0 | 55 | 55 (100%) | tech_debt (2 medium items, no blockers) |
| v1.1 | 27 | 27 (100%) | tech_debt (deployment config + human verification, no blockers) |

### Top Lessons (Verified Across Milestones)

1. Milestone audits before completion catch integration bugs that unit tests miss
2. Enforcement-at-construction patterns (TenantAwareCollection) prevent bug categories, not just instances
3. SUMMARY frontmatter discipline is a recurring gap — needs tooling enforcement, not just process reminders
4. Deployment config must be treated as a first-class deliverable alongside code changes
