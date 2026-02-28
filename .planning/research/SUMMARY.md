# Project Research Summary

**Project:** efOfX Estimation Service — v1.1 Feedback & Quality Milestone
**Domain:** Multi-tenant SaaS estimation platform — brownfield feature addition
**Researched:** 2026-02-28
**Confidence:** HIGH (codebase audited directly; stack verified via PyPI/npm/official docs; pitfalls confirmed against authoritative sources)

## Executive Summary

v1.1 is a brownfield milestone built on top of a shipped v1.0 foundation. The core estimation engine, tenant isolation model, JWT/API key auth, MongoDB Atlas persistence, React widget, and DigitalOcean deployment are all in production. This milestone adds the feedback loop (customer magic-link emails, structured outcome collection), contractor calibration dashboard (accuracy metrics per reference class), distributed LLM caching (Valkey replacing per-process dict), shared library extraction (monorepo workspace setup for future verticals), and a set of non-negotiable tech debt items from the v1.0 audit (INT-04 tenant_id type, INT-05 missing indexes, deprecated MongoDB accessors, ConsultationCTA dead link). The work is well-bounded: almost all required libraries are already declared in `pyproject.toml`; the principal gaps are wiring existing declarations into code and closing the `requirements.txt`/`pyproject.toml` sync.

The recommended approach is incremental and risk-ordered: clean the codebase first (tech debt), then add infrastructure (Valkey), then build user-facing features (feedback email, calibration dashboard), then extract shared libraries last when the codebase is stable. This ordering avoids introducing new features on top of known bugs and ensures the shared library extraction happens against clean, tested code rather than a codebase mid-refactor. All new Python dependencies are already declared; the only truly new frontend dependencies are `recharts` and `@tanstack/react-query` for the new `apps/efofx-dashboard/` Vite app.

The critical risks are concentrated in the magic-link email subsystem: corporate email security scanners (Microsoft Safe Links, Proofpoint, Mimecast) automatically follow every URL in every email, meaning a naive single-use token consumed on HTTP GET will be redeemed before the user clicks. The mitigation is a two-step model where GET to the feedback URL is idempotent (renders form, does not consume token) and only the form POST consumes the token. A secondary risk is email deliverability: a new sending domain with no reputation will immediately land in spam. Both of these must be resolved in the first story of the feedback phase — not retrofitted later.

---

## Key Findings

### Recommended Stack

The v1.1 stack is almost entirely the existing v1.0 stack. No new Python dependencies are needed beyond what is already declared in `pyproject.toml`. The principal code-level work is syncing `requirements.txt` to match `pyproject.toml` (specifically: `fastapi-mail==1.6.2` and `valkey>=6.1.0,<7.0.0`), then wiring those declared libraries into the application.

The one genuinely new addition is a frontend dashboard application (`apps/efofx-dashboard/`) built with Vite + React 19 (matching the existing widget) that requires `recharts@^3.7.0` for calibration charts and `@tanstack/react-query@^5.0.0` for async data fetching. Both packages are React 19 compatible and align with the existing Tailwind CSS convention. No additional charting libraries should be added — Recharts is SVG-based, Tailwind-compatible, and has no vendor lock-in.

**Core technologies (v1.1 additions):**
- `valkey>=6.1.0` (already in pyproject.toml): Distributed LLM response cache replacing the per-process dict in `llm_service.py`. Required for multi-worker correctness. DigitalOcean Managed Valkey at $15/month.
- `fastapi-mail==1.6.2` (already in pyproject.toml): Magic-link email dispatch. Reuses the SMTP config already wired in `auth_service.py`. No new SMTP env vars needed; only `MAGIC_LINK_SECRET` and `FEEDBACK_FORM_URL` are new config keys.
- `recharts@^3.7.0` + `@tanstack/react-query@^5.0.0` (new, frontend only): Calibration dashboard charts and server state management in the new `apps/efofx-dashboard/` app.
- `secrets.token_urlsafe(32)` (Python stdlib): Magic link token generation. No external library needed; MongoDB provides single-use invalidation via atomic `findOneAndUpdate`.
- uv workspace + npm workspaces: Monorepo structure for shared library extraction. No new package managers; uses existing uv and npm tooling.

See `.planning/research/STACK.md` for full version compatibility matrix and alternatives analysis.

### Expected Features

v1.1 is feature-scoped. All P1 features are required to ship. P2 features are deferred until post-v1.1 validation.

**Must have (table stakes):**
- Magic link token generation and email send — the feedback loop cannot start without this; customers need a zero-friction path to submit outcomes
- Customer feedback form with structured outcome fields (actual_cost, actual_timeline, discrepancy reason enum) — structured data only; free text alone is not tunable
- Token validation endpoint — idempotent GET (renders form without consuming token), single-use POST (consumes token on form submit)
- Feedback document storage with estimate snapshot and reference class linkage — immutable snapshot; do not overwrite original estimate
- Calibration metrics calculation — mean variance, accuracy buckets (within 10/20/30%), per-reference-class breakdown; minimum 10 real outcomes threshold before any metric is displayed
- Contractor calibration dashboard (new `apps/efofx-dashboard/`) — read-only, tenant-scoped; contractors never see individual customer responses
- INT-04 fix (EstimationSession tenant_id type) — risk of silent tenant isolation bugs before feedback data accumulates under wrong type
- INT-05 fix (widget analytics missing compound indexes) — performance correctness before calibration dashboard queries the same collections
- Valkey cache migration — multi-worker deployments are broken without shared cache
- ConsultationCTA wiring — dead UI element in production; one-line fix requiring a product decision on destination URL
- Deprecated MongoDB accessor removal — code clarity before shared library extraction
- Shared backend utilities extraction (`packages/efofx-shared/`) — crypto, validation, calculation utils; stdlib only, zero FastAPI/Motor imports
- Shared frontend component extraction (`packages/efofx-ui/`) — EstimateCard, ChatBubble, TypingIndicator; no widget-specific state
- YAGNI pass — delete unused code paths before extraction compounds the maintenance burden

**Should have (competitive differentiators within v1.1 scope):**
- Estimate-contextualized feedback email — pre-populating the email with the original estimate range dramatically improves response quality vs. cold surveys
- Reference class level accuracy labels (when N >= 5 per class) — contractors can use per-category accuracy as a sales signal

**Defer (v1.2+):**
- Automated contractor notification after customer feedback submission — useful only after feedback volume makes it worthwhile
- Temporal calibration trend charts — requires >30 days of data per tenant to be meaningful
- Synthetic data weight adjustment workflow — requires N >= 10 per reference class for statistical confidence
- IT/dev vertical reference classes — uses shared library from v1.1; this is the next milestone, not v1.1

**Anti-features (never build in v1.x):**
- Automated LLM prompt tuning from feedback — reliability disaster without a human-reviewed evaluation harness
- Email drip campaigns for non-responders — harms contractor-customer relationships; requires CAN-SPAM compliance work
- Customer login accounts — doubles auth surface for one-time quote requestors; magic link is the correct model

See `.planning/research/FEATURES.md` for full prioritization matrix and dependency graph.

### Architecture Approach

v1.1 follows the existing FastAPI layered service pattern strictly. The middleware stack, `TenantAwareCollection` isolation model, and dual-auth (JWT + API key) are untouched. Three new services are added (`FeedbackEmailService`, `CalibrationService`, `ValkeyCache`), one existing service is modified (`LLMService`), and a new frontend app (`apps/efofx-dashboard/`) is scaffolded. Two new shared packages (`packages/efofx-shared/` Python, `packages/efofx-ui/` React) are extracted using uv workspaces and npm workspaces respectively.

**Major components (new in v1.1):**
1. `FeedbackEmailService` — HMAC-signed magic link token generation + `fastapi-mail` dispatch (fire-and-forget via `asyncio.create_task()`). Separate from `FeedbackService` because SMTP failure semantics must not affect feedback CRUD.
2. `CalibrationService` — MongoDB aggregation pipeline computing accuracy metrics per reference class. Critical: `$lookup` joins to `estimates` collection must manually scope `tenant_id` in the inner pipeline — `TenantAwareCollection.aggregate()` only scopes the source collection, not the join target.
3. `ValkeyCache` — module-level singleton async connection pool. Must wrap all calls in try/except for graceful degradation (Valkey outage falls back to uncached LLM call, not a 500 error).
4. `magic_link_tokens` MongoDB collection — TTL index on `expires_at`, unique index on `token_hash`, two-step redemption schema with distinct `opened_at` and `used_at` fields.
5. `apps/efofx-dashboard/` — New Vite + React 19 app with Recharts calibration charts and React Query for API state. JWT-authenticated, contractor-facing, read-only.
6. `packages/efofx-shared/` + `packages/efofx-ui/` — Extracted pure utilities. The Python package must have zero imports from any `apps/` directory; the React package must contain no estimation-domain logic.

The build order is mandated by dependency analysis: Phase 1 tech debt → Phase 2 Valkey → Phase 3 feedback email → Phase 4 calibration → Phase 5 shared extraction.

See `.planning/research/ARCHITECTURE.md` for full data flow diagrams, code patterns, anti-patterns, and component detail.

### Critical Pitfalls

1. **Email security scanners consume GET-based magic link tokens** — Corporate email scanners (Microsoft Safe Links, Proofpoint, Mimecast) auto-follow every URL in every email. If the token is consumed on HTTP GET, the scanner redeems it before the user clicks. Avoid by making GET to the feedback URL idempotent (renders form, sets `opened_at`, does NOT set `used_at`); only the form POST consumes the token. The `opened_at`/`used_at` distinction must be in the schema before the first token document is written.

2. **New sending domain with no reputation goes to spam immediately** — A cold domain sending time-limited magic links to corporate inboxes triggers every spam heuristic. Use a dedicated transactional email provider (Resend, Postmark, or SendGrid) with established IP reputation, configure SPF/DKIM/DMARC before the first send, and test delivery to Gmail, Outlook, and a GSuite corporate inbox. This is an infrastructure prerequisite, not a code story.

3. **FastAPI BackgroundTasks silently drops failed email sends** — The default BackgroundTasks exception handler swallows errors with no retry and no monitoring surface. Magic link tokens must be persisted to MongoDB before the background task fires (not conditional on email success). The task body must have explicit try/except with structured logging and a failed-send record written to the DB.

4. **Calibration dashboard shows metrics before statistically meaningful sample sizes** — Displaying calibration metrics based on 3 data points trains contractors to distrust the system. Enforce a minimum of 10 confirmed real outcomes per tenant before any metric renders. Show a progress indicator below the threshold. Tag all existing synthetic reference class documents with `data_source: "synthetic"` before v1.1 ships; calibration queries must filter exclusively to `data_source: "real"`.

5. **Valkey cross-tenant cache key collision** — If LLM cache keys are not prefixed with `tenant_id`, a cache hit for Tenant A's estimate can be served to Tenant B. All cache keys must be prefixed: `v1:llm:{tenant_id}:{sha256_hash}`. Use `hashlib.sha256` (deterministic across processes), never Python's `hash()` (randomized per process start). Serialize cached values as JSON, never pickle.

6. **Shared package circular import discovered after second vertical tries to use it** — The shared package acquires implicit imports from `apps/efofx_estimate` over time. By the time the second vertical tries to install `efofx-shared` standalone, it has hidden transitive dependencies on FastAPI and Motor. Enforce zero `apps/` imports in the shared package. Write a CI test that installs the package in a fresh venv with only declared dependencies and runs `import efofx_shared` — failure means a circular import leaked in.

---

## Implications for Roadmap

The dependency graph mandates a specific phase order. No phase can be safely reordered because each phase either fixes issues that would complicate subsequent work or provides capabilities the next phase requires.

### Phase 1: Tech Debt & Foundation Cleanup

**Rationale:** The v1.0 audit identified concrete correctness bugs (INT-04 tenant_id type, INT-05 missing indexes) and code quality issues (deprecated MongoDB accessors, ConsultationCTA dead link) that would complicate or mask bugs in subsequent feature work. Extracting shared libraries into a messy codebase embeds the debt permanently. This phase has zero external dependencies — it can start immediately without infrastructure provisioning.

**Delivers:** A codebase where all known bugs are fixed, deprecated patterns are removed, and dead code is eliminated. The shared extraction in Phase 5 will extract clean, tested code rather than working around dead code.

**Addresses features from FEATURES.md:** INT-04 fix, INT-05 fix, deprecated accessor removal, ConsultationCTA wiring, YAGNI pass.

**Avoids pitfalls:** "Deprecated accessor removal breaking test scripts" — full test suite including `scripts/` must run after removal. "YAGNI removal of in-use feature" — MongoDB usage check before any removal (query for 90-day usage, only remove if count == 0).

**Research flag:** Standard patterns — no research-phase needed. Mechanical cleanup with well-defined acceptance criteria from the v1.0 audit.

---

### Phase 2: Valkey Infrastructure

**Rationale:** The per-process `_response_cache: dict` in `llm_service.py` is a documented correctness bug in multi-worker deployments. Fixing this before adding more user load (feedback emails triggering estimation calls, calibration queries) prevents the feedback loop from running on a broken cache. Provisioning DigitalOcean Managed Valkey is a prerequisite — the infrastructure must be live before the application can connect to it.

**Delivers:** A shared distributed LLM response cache that works correctly across all Gunicorn workers. Cache survives app restarts and is scoped per tenant to prevent cross-tenant collisions.

**Uses from STACK.md:** `valkey>=6.1.0` (already in pyproject.toml), `fakeredis>=2.0.0` for unit tests (already in pyproject.toml dev extras), `VALKEY_URL` config already present in `app/core/config.py`.

**Implements architecture:** `ValkeyCache` singleton module (`app/services/valkey_cache.py`) with graceful fallback; `llm_service.py` modification (same SHA-256 cache key logic, Valkey replacing in-process dict).

**Avoids pitfalls:** Cross-tenant cache key collision (prefix all keys with `tenant_id`), non-deterministic cache keys (use `hashlib.sha256`, never `hash()`), Valkey outage crashes (try/except with fallback to live LLM call), pickle serialization breaks (JSON only, never pickle Pydantic models), TTL not set (always set explicit TTL; no unbounded cache growth).

**Research flag:** Standard patterns — no research-phase needed. Official Valkey docs are authoritative. One open question to verify before starting: whether slowapi's `limits` library accepts the `valkeys://` TLS URL scheme used by DO Managed Valkey. If not, the workaround (`redis://...?ssl=true`) is known.

---

### Phase 3: Feedback Email & Magic Links

**Rationale:** This is the central user-facing feature of v1.1. It depends on Phase 1 (clean feedback model, no deprecated accessor bugs) but does not depend on Phase 4 (calibration). It must be built before calibration because calibration metrics can only be computed after real feedback data exists. Email infrastructure (transactional provider account, SPF/DKIM/DMARC records) must be verified as the first story in this phase — before any magic link code is written. Getting this wrong silently breaks the entire feedback loop.

**Delivers:** Contractors can trigger a customer feedback email after an estimate is delivered. Customers receive a contextualized, time-limited magic link, fill in actual project outcomes on a stateless public form, and the data is stored against the estimate snapshot for calibration.

**Addresses features from FEATURES.md:** Magic link token generation, email send, token validation (idempotent GET / single-use POST), feedback form (customer-facing), feedback document storage with estimate snapshot and reference class linkage.

**Avoids pitfalls:** Email security scanners consuming GET-based tokens (two-step GET/POST model committed before any token doc schema is finalized), email deliverability (transactional provider + SPF/DKIM/DMARC + inbox testing as the first story), silent background task failure (explicit try/except + failed-send record in DB).

**Research flag:** Needs research-phase during planning for email deliverability specifics. Which transactional email provider integrates cleanly with `fastapi-mail`? Does the efofx.ai domain have existing SPF/DKIM records? Resend, Postmark, and SendGrid each have different SMTP API configuration. This is operations-heavy, not code-heavy, and the wrong choice here silently breaks the entire feedback loop.

---

### Phase 4: Calibration Dashboard

**Rationale:** Calibration metrics are only meaningful once feedback data exists (Phase 3). This phase builds the aggregation layer and the new `apps/efofx-dashboard/` frontend. The `$lookup` tenant scoping requirement (explicitly scoping `tenant_id` in the inner aggregation pipeline, not relying on `TenantAwareCollection` alone) is a security-critical constraint that must be in the task acceptance criteria for every calibration story.

**Delivers:** Contractors see their historical estimate accuracy: mean variance, accuracy buckets (within 10/20/30% of actual), per-reference-class breakdown. The dashboard shows a progress indicator ("X more outcomes needed") until the minimum 10-outcome threshold is reached. Synthetic reference class data is excluded from all metric calculations.

**Uses from STACK.md:** `recharts@^3.7.0` + `@tanstack/react-query@^5.0.0` in the new `apps/efofx-dashboard/` Vite app. uv workspace root `pyproject.toml` needed before this phase if not already set up in Phase 5 prep.

**Implements architecture:** `CalibrationService` with tenant-scoped MongoDB aggregation pipeline, calibration Pydantic models (`app/models/calibration.py`), new calibration endpoints (`/calibration/summary`, `/calibration/accuracy`), new `apps/efofx-dashboard/` Vite frontend app.

**Avoids pitfalls:** Calibration metrics with insufficient or synthetic data (minimum 10-outcome threshold enforced in service; `data_source: "real"` filter required — the `data_source` field migration on existing synthetic data is the first story of this phase), tenant-unscoped `$lookup` (inner pipeline must include its own `tenant_id` match).

**Research flag:** Standard patterns for MongoDB aggregation and Recharts. One lightweight check: verify Recharts + Tailwind integration in a new Vite app before starting the dashboard UI stories, since this is the first time the team will use Recharts.

---

### Phase 5: Shared Library Extraction

**Rationale:** This phase is deliberately last. Extracting shared libraries after the codebase has been cleaned (Phase 1) and features have stabilized (Phases 3-4) minimizes the risk of extracting code that immediately needs to change. A boundary document (what is allowed in shared, what is not) must be the first deliverable of this phase — before any code moves. The Python package must be installable in a fresh venv with only stdlib; the React package must contain no estimation-domain logic.

**Delivers:** `packages/efofx-shared/` (Python crypto, validation, calculation utilities) and `packages/efofx-ui/` (EstimateCard, ChatBubble, TypingIndicator) extracted as workspace packages. uv workspace root `pyproject.toml` and npm workspaces `package.json` configured. The IT/dev vertical can be initialized in v1.2 as a configuration-plus-reference-classes addition, not a rewrite.

**Avoids pitfalls:** Shared package circular import (zero imports from `apps/`; CI test verifies fresh-venv install), shared package published to public registry accidentally (mark `private: true` in `package.json`; do not configure PyPI publishing in `pyproject.toml` for internal packages), widget embed behavior change after extraction (treat widget embed API as versioned contract; test existing embed script against refactored widget before releasing).

**Research flag:** Standard patterns — no research-phase needed. uv workspaces and npm workspaces are well-documented in official docs. One open question to resolve before starting: whether the DigitalOcean App Platform build uses `pyproject.toml` with `pip install -e .` or `requirements.txt`. If it uses `requirements.txt`, editable install of `packages/efofx-shared` must be added to `requirements.txt` explicitly.

---

### Phase Ordering Rationale

- **Tech debt first:** INT-04 (tenant_id type) and INT-05 (missing indexes) are correctness bugs that could silently corrupt calibration data if feedback data accumulates under the wrong type. Fixing these before any feedback data exists eliminates the risk of a migration against real production data.
- **Valkey before user-facing features:** Multi-worker cache correctness is a prerequisite for reliable LLM estimation. Shipping more users onto a broken per-process cache while adding the feedback loop means every feedback-triggered estimation call risks cache misses on all but one worker.
- **Feedback before calibration:** Calibration metrics require real feedback data. The calibration service can be built against synthetic test data in development, but the minimum-threshold logic assumes real data has started flowing.
- **Extraction last:** Shared library extraction is a pure refactoring milestone with no user-visible impact. Doing it last means the code being extracted is already tested, clean, and stable. Doing it earlier risks mid-milestone churn if feature work requires changing the code being extracted.

---

### Research Flags

**Phases likely needing `/gsd:research-phase` during planning:**

- **Phase 3 (Feedback Email):** Email deliverability infrastructure is the highest-risk area in the entire milestone. Before writing the FeedbackEmailService integration story, verify: (1) which transactional email provider integrates cleanly with `fastapi-mail`, (2) whether the efofx.ai domain has any pre-existing SPF/DKIM records, (3) SPF/DKIM/DMARC configuration steps for the chosen provider. This is operations-heavy, not code-heavy, and the wrong choice silently breaks the entire feedback loop.

**Phases with standard patterns (skip research-phase):**

- **Phase 1 (Tech Debt):** Mechanical cleanup. All acceptance criteria are defined in the v1.0 audit (CONCERNS.md, v1.0-MILESTONE-AUDIT.md). No research needed.
- **Phase 2 (Valkey):** Official Valkey docs are authoritative. Integration patterns are documented in STACK.md and ARCHITECTURE.md with working code examples. Verify the `valkeys://` TLS URL scheme with slowapi before starting — that is a verification task, not a research sprint.
- **Phase 4 (Calibration):** MongoDB aggregation patterns are well-documented. The calibration math is straightforward. The tenant-scoped `$lookup` pattern is documented in ARCHITECTURE.md with a working code example.
- **Phase 5 (Shared Extraction):** uv workspaces and npm workspaces are documented in official sources. Boundary definition is the risk, not the tooling — resolve with a one-page document before moving any code.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All Python deps verified via PyPI; all npm deps verified via npm registry and peerDependency inspection. The only uncertainty is runtime verification of `valkeys://` with slowapi — marked as an open question to verify before Phase 2 begins. |
| Features | MEDIUM | Core feature requirements are HIGH confidence (from PRD.md and v1.0 audit). Construction-specific calibration accuracy expectations ("what good looks like" for contractors) are MEDIUM confidence — no direct industry precedent found. The 10-outcome minimum threshold is a defensible recommendation but not an industry-verified constant; treat as a product decision to validate with the stakeholder. |
| Architecture | HIGH | Existing v1.0 source was read directly. All integration patterns are based on actual code, not inference. The CalibrationService `$lookup` tenant scoping requirement is verified against MongoDB documentation. The magic link two-step redemption model is verified against documented email scanner behavior from official vendor docs. |
| Pitfalls | HIGH | Critical pitfalls verified against official docs and multiple corroborating sources. Email scanner GET-consumption is documented by Supabase, Mimecast (official), and community discussion. BackgroundTasks silent failure is documented in FastAPI official docs. Valkey cross-tenant cache key collision is a structural security requirement, not speculative. |

**Overall confidence:** HIGH

### Gaps to Address

- **Email provider selection:** The research recommends a transactional email provider but does not specify which one. Resolution: verify which provider integrates cleanly with `fastapi-mail` and whether the efofx.ai domain has pre-existing SPF/DKIM records. This must be resolved as the first story of Phase 3 before any magic-link code is written.

- **`requirements.txt` vs `pyproject.toml` build authority on DigitalOcean:** If DO App Platform uses `requirements.txt`, the sync gap (missing `fastapi-mail`, `valkey`) causes production missing-dependency failures. Resolution: inspect `.do/app.yaml` build command and DO App Platform docs before Phase 1 begins.

- **slowapi `valkeys://` TLS URL scheme compatibility:** The `limits` library used by slowapi may require `redis://` scheme syntax even for a Valkey host on TLS. DO Managed Valkey uses `valkeys://`. Resolution: test the connection string against a provisioned DO Managed Valkey instance before Phase 2 code changes begin.

- **Minimum feedback threshold for calibration:** The recommended 10-outcome minimum is defensible but is a product decision, not a statistical constant. Resolution: validate with the product stakeholder before Phase 4 begins, as it affects acceptance criteria for the "insufficient data" placeholder state.

- **ConsultationCTA destination URL:** The CTA wiring (Phase 1) requires a business decision on the contractor contact flow destination. Resolution: confirm the destination URL before Phase 1 begins — the code fix is one line, but the product decision must come first.

---

## Sources

### Primary (HIGH confidence)

- efOfX codebase — direct source read of `apps/efofx-estimate/app/` (authoritative; all architecture integration patterns based on actual code)
- efOfX PRD — `docs/PRD.md` (authoritative product requirements)
- efOfX v1.0 audit — `.planning/milestones/v1.0-MILESTONE-AUDIT.md` and `.planning/codebase/CONCERNS.md` (authoritative; confirms INT-04, INT-05, and all tech debt items)
- PyPI — `fastapi-mail==1.6.2`, `valkey==6.1.1`, `fakeredis` — verified 2026-02-28
- npm — `recharts@3.7.0` peerDependencies (React 19 confirmed), `@tanstack/react-query@5` — verified 2026-02-28
- DigitalOcean official docs — Managed Valkey product page, DO Blog: Introducing Managed Valkey (confirms Redis → Valkey migration June 2025)
- Valkey.io official docs — valkey-py async connection pool, migration guide
- MongoDB documentation — `$lookup` with pipeline sub-array and `let` variable binding for correlated lookups
- FastAPI official docs — BackgroundTasks behavior and exception handling
- Supabase Docs — OTP verification failures / email prefetching (official documentation of scanner behavior)
- Mimecast — URL pre-scanning official documentation
- Amazon Forecast Docs — P50/P80 accuracy metric definitions
- uv workspace docs — astral.sh/uv/concepts/workspaces (official)

### Secondary (MEDIUM confidence)

- Baytech Consulting — Magic Links UX, Security and Growth (2025)
- WorkOS — A Guide to Magic Links
- guptadeepak.com — Magic Link Security Best Practices
- Cultivate Labs — What is Forecast Calibration
- AskYazi — Survey Response Rates Guide (NPS and Post-Interaction)
- Monorepo.tools — Monorepo Explained
- Feature-Sliced Design — Frontend Monorepo Guide 2025
- Mailgun — Domain Warm-up Reputation Guide
- Milan Jovanovic — Solving the Distributed Cache Invalidation Problem with Redis
- Gmail/Hacker News discussion (2021) — email clients pre-fetching URLs in emails (confirms scanner behavior)
- DNV — Terminology Explained P10, P50, P90 (engineering standards organization)

### Tertiary (LOW confidence)

- Feedback → reference class linkage for calibration tuning — inferred from RCF engine design, not externally verified. Treat as a product assumption to validate once feedback data starts flowing.
- Construction industry calibration benchmarks — no direct industry study found. The PRD target of 70% of estimates within 20% of actual is a product goal, not an industry-validated baseline. Do not present this to contractors as an industry standard.

---

*Research completed: 2026-02-28*
*Ready for roadmap: yes*
