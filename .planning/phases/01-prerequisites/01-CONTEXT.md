# Phase 1: Prerequisites - Context

**Gathered:** 2026-02-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix known bugs and replace abandoned dependencies so Phase 2 can be built on a stable foundation. Requirements PRQT-01 through PRQT-05. No new features — only fixes, replacements, and cleanup.

</domain>

<decisions>
## Implementation Decisions

### Structured Output Schema

**Domain-agnostic estimation model** — the system supports multiple project domains, each with its own reference class model containing domain-specific cost categories and schedule phases.

**Three initial domains:**

**Construction:**
- Cost Categories: Permits & Approvals, Site Preparation, Materials, Labor (by trade), Equipment & Rentals, Subcontractors, Inspections & Compliance, Contingency
- Schedule Phases: Permitting, Site Prep, Foundation, Rough Construction, MEP, Finishing, Inspection & Closeout

**IT Projects:**
- Cost Categories: Infrastructure, Software Licensing, Security & Compliance, Integration, Observability & Monitoring, Tooling & Automation, Labor (internal + contractors), Training & Change Management, Contingency
- Schedule Phases: Discovery & Assessment, Architecture & Design, Procurement, Implementation, Integration & Testing, Migration & Cutover, Stabilization

**Software Development:**
- Cost Categories: UI/Frontend, Backend/API, Infrastructure & DevOps, Integration (3rd party), Testing & QA, Observability, Documentation, Labor, Contingency
- Schedule Phases: Requirements & Design, Core Development, Integration, Testing & QA, Deployment & DevOps, Stabilization & Hardening

**Estimate output structure:**
- P50/P80 ranges calculated **per cost category** (not just totals)
- P50/P80 duration ranges **per schedule phase** (not just total timeline)
- Adjustment factors shown as **named multipliers** (e.g., "Urban premium: 1.15x") — transparent, not baked in
- **Numeric confidence score** (0-100) reflecting how much information the LLM had to work with
- **Explicit assumptions list** — each estimate includes the assumptions the LLM made
- **Final estimate only** in output — no raw RCF engine data exposed (stays internal)
- Inapplicable categories **omitted** (not zero-filled) — category set varies per estimate

**Domain identification:**
- Tenant sets a **default domain** in their profile
- LLM can **detect and override** the domain from conversation context
- **Single domain per estimate** — multi-domain projects create separate estimates

**New domains:**
- **Platform-defined only** — efOfX adds domains with proper reference data
- Contractors can **suggest** new domains for the platform to add

**Schema enforcement:**
- Estimate schema defined as **Pydantic models** (required for OpenAI v2 structured outputs)

### Tenant Isolation Fix

- **Patch queries only** — add tenant_id filter to all rcf_engine.py queries. Minimal change. Phase 2 builds the proper TenantAwareCollection middleware.
- **Automated test required** — write a test that creates two tenants, inserts data, and asserts zero cross-tenant leakage. Runs in CI.
- **Platform data in separate collection** — synthetic reference classes live in their own MongoDB collection, distinct from tenant-owned data.
- **Auto-include platform data** — queries automatically merge tenant data + platform reference data. No opt-in parameter needed.

### Dependency Migration

- **No Redis/Valkey** — remove all existing Redis client code, connection config, and caching logic. MVP runs without Redis. Reserve the option to add it later for performance.
- **MongoDB-based rate limiting** for Phase 2 — store counters in MongoDB with TTL indexes instead of Redis/Valkey.
- **Full refactor to OpenAI v2 patterns** — use structured outputs, new client patterns, proper error types. Don't just make old code compile.
- **Python 3.11 (unpinned)** — specify 3.11 in DigitalOcean App Platform config, let the platform pick the latest patch.
- **Stick to requirements only** — no additional dependency audit or cleanup beyond PRQT-01 through PRQT-05 (plus Redis removal).
- **Remove Redis entirely** — if Redis code exists, remove it. Clean slate. Don't leave dormant Redis code.

### Claude's Discretion

- How much of the full domain-aware structured output schema to implement in Phase 1 vs. deferring to Phase 3 (assess what the RCF engine can support now)
- Exact Pydantic model structure for the estimate schema
- Test framework and test organization for the tenant isolation test
- Order of dependency replacements

</decisions>

<specifics>
## Specific Ideas

- Domain models should be consistent within a domain so calibration data aggregates meaningfully across estimates
- "Reserve the option to add Redis later for performance" — architecture should not preclude it
- Pydantic models for structured output will be consumed by OpenAI v2's structured output feature

</specifics>

<deferred>
## Deferred Ideas

- Multi-domain estimates (project spanning Construction + Software Dev) — start with single domain, revisit if real users need it
- Contractor-created custom domains — platform-defined only for now
- Full dependency audit — defer to Phase 6 (Code Quality and Hardening)
- Redis/Valkey for caching — add back if performance requires it

</deferred>

---

*Phase: 01-prerequisites*
*Context gathered: 2026-02-26*
