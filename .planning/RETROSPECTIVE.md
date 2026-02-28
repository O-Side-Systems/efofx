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

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 77 | 5 | Established GSD workflow with milestone audit → gap closure pattern |

### Cumulative Quality

| Milestone | Requirements | Satisfied | Audit Status |
|-----------|-------------|-----------|--------------|
| v1.0 | 55 | 55 (100%) | tech_debt (2 medium items, no blockers) |

### Top Lessons (Verified Across Milestones)

1. Milestone audits before completion catch integration bugs that unit tests miss
2. Enforcement-at-construction patterns (TenantAwareCollection) prevent bug categories, not just instances
