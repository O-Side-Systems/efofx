# Phase 9: Shared Library Extraction - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Extract shared backend (Python) and frontend (React) code into workspace packages that any future vertical can consume. Create a boundary document, set up CI isolation tests, document code quality standards, and audit the codebase for conformance. The IT/dev vertical (v1.2) should be able to initialize without copying code from the estimation app.

</domain>

<decisions>
## Implementation Decisions

### Package boundaries
- Extract ALL reusable utilities to packages/efofx-shared/, not just multi-vertical ones — anything not estimation-specific moves
- Mirror the current app structure: packages/efofx-shared/efofx_shared/core/, utils/, models/ — familiar layout
- Immediate re-import: move code to shared package and update app imports in the same phase — one clean cut, no deprecation period
- Boundary document format: decision table (Module | Location | Rationale) covering every module with clear reasoning
- Use uv workspaces for Python monorepo linking — no PyPI publishing, changes immediately available

### Component extraction scope
- Audit ALL components across efofx-widget and efofx-dashboard — extract everything reusable, not just the 3 named components
- The named 3 (EstimateCard, ChatBubble, TypingIndicator) are the minimum; additional generic components should also move
- CSS modules co-located with each component (.module.css files) — scoped by default, overridable by consuming apps
- Use npm workspaces for frontend monorepo linking — no npm publishing, workspace protocol in package.json

### Code quality standards
- Comprehensive STANDARDS.md at repo root (alongside CLAUDE.md) covering: code style, file/folder structure, testing patterns, documentation (docstrings, README), error handling patterns, logging conventions
- Audit the ENTIRE codebase for conformance — not just the extracted packages
- Fix ALL violations found during the audit in this phase — leave the codebase fully conformant, not just catalogued

### CI isolation strategy
- CI checks BOTH packages: Python import isolation AND TypeScript build isolation
- Platform: DigitalOcean App Platform (already used for hosting — keep everything in one place)
- Create CI from scratch — no existing CI workflows to extend
- Full test suite on every PR: isolation tests + pytest + npm build — catches regressions from extraction refactoring

### Claude's Discretion
- Exact modules to extract (determined by codebase audit)
- uv workspace configuration details
- npm workspace configuration details
- CI workflow file structure and job organization
- Order of extraction operations

</decisions>

<specifics>
## Specific Ideas

- Python package name: efofx-shared (importable as efofx_shared)
- Frontend package name: efofx-ui (or @efofx/ui for npm scope)
- Boundary document should be a markdown table that resolves ambiguous future extraction decisions — not just principles, but concrete module-by-module decisions
- The shared Python package must install in a fresh virtualenv with only its declared dependencies — no FastAPI, Motor, or apps/ imports leaking in

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 09-shared-library-extraction*
*Context gathered: 2026-03-15*
