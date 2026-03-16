# Phase 5: Tech Debt & Foundation Cleanup - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix all v1.0 audit items: tenant_id type mismatch, missing collection indexes, deprecated MongoDB accessors, broken ConsultationCTA button, dead code removal, and dependency sync. No new features — correctness and cleanup only.

</domain>

<decisions>
## Implementation Decisions

### ConsultationCTA destination (DEBT-04)
- Button opens an inline contact form within the widget (not a modal, not an external link)
- Form fields: name, email, phone number, free-text message
- Submission stores a document in widget_leads collection AND sends email notification to the contractor
- Form labels and placeholder text support locale-based defaults (en + es at launch) with per-tenant overrides in tenant config
- Per-tenant overrides take priority over locale defaults

### Dead code / YAGNI pass (DEBT-05)
- Conservative scope: remove only clearly dead code — unused imports, unreachable functions, commented-out blocks
- Do NOT refactor working code or simplify over-engineered patterns — if it runs, leave it
- Covers both Python backend and React/JS frontend
- All commented-out code blocks are removed entirely (git history preserves them)
- Unused dependencies in pyproject.toml and package.json are removed

### Tenant ID migration (DEBT-01)
- Fix the code to use the correct tenant identifier type going forward
- Write a migration to update existing EstimationSession documents to match the corrected type
- Migration runs automatically on deploy (application startup), not as a manual script
- Migration is idempotent — safe to re-run, no dry-run mode needed

### Compound indexes (DEBT-02)
- widget_analytics and widget_leads indexes created via ensure_index on application startup
- Idempotent — always in sync with code, no separate migration script

### Claude's Discretion
- Deprecated accessor removal approach (DEBT-03) — straightforward deletion
- Dependency sync strategy (DEBT-06) — match requirements.txt to pyproject.toml
- Migration script structure and error handling
- Contact form validation rules and error messages
- Email notification template for contractor leads

</decisions>

<specifics>
## Specific Ideas

- User wants all user-facing messaging to be configurable and localizable — applied to the contact form for this phase (locale defaults + per-tenant overrides)
- English + Spanish locale support at launch for contact form labels

</specifics>

<deferred>
## Deferred Ideas

- Full widget localization (all user-facing messaging across the entire widget, not just the contact form) — future phase
- Additional locale support beyond en + es — add as needed in future phases

</deferred>

---

*Phase: 05-tech-debt-foundation-cleanup*
*Context gathered: 2026-02-28*
