# Project Research Summary

**Project:** efOfX Estimation Service — Epics 3-7
**Domain:** Multi-tenant B2B SaaS estimation platform with LLM integration, white-label widget, and self-improving feedback loop
**Researched:** 2026-02-26
**Confidence:** MEDIUM-HIGH

## Executive Summary

efOfX is a brownfield multi-tenant SaaS platform that converts a working construction estimation engine (Epics 1-2) into a commercially deployable product through four capability pillars: multi-tenant security (Epic 3), LLM conversational estimation (Epic 4), white-label embeddable widget (Epic 5), and a self-improving feedback/calibration loop (Epic 6). The research consensus is clear: JWT-based tenant isolation is the keystone dependency that unlocks every other capability, and it must be implemented correctly from the start — retrofitting multi-tenancy into a live system is a high-cost, high-risk operation that can be avoided entirely by building the isolation layer first.

The recommended technical approach is FastAPI middleware-enforced tenant isolation (not service-layer filtering), Fernet encryption with per-tenant HKDF key derivation for BYOK OpenAI keys, OpenAI SDK v2 with per-request key injection for LLM calls, and Shadow DOM isolation with Vite IIFE bundling for the embeddable widget. The platform's competitive differentiation — probabilistic estimates, white-label embedding, and a self-improving feedback loop — is achievable with these choices. Several critical dependency upgrades are required before Epic 3 begins: `python-jose` and `passlib` are both abandoned packages that must be replaced with `PyJWT` and `pwdlib` respectively, and the OpenAI SDK must be upgraded from v1.51 to v2.x.

The primary risks are a known tenant isolation gap in `rcf_engine.py` (cross-tenant reference class queries), a stubbed LLM parser that bills tenants without delivering value, an existing `NameError` in `security.py` that breaks API key auth, and the potential for synthetic training data to contaminate the calibration loop if not explicitly tagged before Epic 6 begins. None of these are unsolvable — all have clear mitigations documented in the research — but each must be addressed in the correct phase to avoid compounding technical debt.

---

## Key Findings

### Recommended Stack

The existing stack (FastAPI, MongoDB Atlas with Motor, React 19, Vite, DigitalOcean) is sound and requires no replacement. Epics 3-7 add targeted libraries for specific capabilities. Four packages in the current `requirements.txt` are abandoned or severely outdated and must be replaced before building on top of them: `python-jose` (abandoned 2021, security CVE) → `PyJWT[crypto]==2.11.0`; `passlib[bcrypt]` (abandoned 2020, broken with bcrypt 5.0+) → `pwdlib[argon2]==0.3.0`; `openai==1.51.0` (v1 end-of-life) → `openai>=2.20.0,<3.0.0`; `redis==5.0.8` → `valkey>=6.1.0` (DigitalOcean deprecated Managed Redis in June 2025, now runs Managed Valkey).

**Core technology additions:**
- `PyJWT[crypto]==2.11.0`: JWT auth with tenant_id claims — actively maintained, FastAPI official docs migrated to this from python-jose
- `pwdlib[argon2]==0.3.0`: Password hashing — Argon2 is memory-hard and GPU-resistant, superior to bcrypt for multi-tenant environments
- `cryptography>=46.0.0` (upgrade): Fernet symmetric encryption for BYOK key storage at rest
- `valkey>=6.1.0`: Caching and rate limiting backend — drop-in redis-py fork required for DigitalOcean Managed Valkey
- `slowapi==0.1.9`: Per-tenant, per-tier rate limiting middleware with Valkey backend
- `openai>=2.20.0,<3.0.0`: LLM client — v2 `AsyncOpenAI` supports per-request api_key override, enabling BYOK without module-level key storage
- `fastapi-mail==1.6.2`: Async SMTP for magic link emails — provider-agnostic, FastAPI-native
- `react-shadow==20.6.0`: React Shadow DOM root creation for widget isolation
- Vite `build.lib` IIFE mode (existing): Widget bundling as single `<script>` — no separate bundler needed
- `jinja2>=3.1.0`: Prompt template rendering for git-versioned prompt files

**What NOT to use:** LiteLLM (over-engineered for single-provider MVP), CSS-in-JS inside Shadow DOM (150-200ms overhead), iframes for widget embedding (cross-origin complexity, mobile UX degradation), or automated RLHF fine-tuning before sufficient real outcome data volume exists.

See `.planning/research/STACK.md` for full installation commands, version compatibility matrix, and open questions.

### Expected Features

The feature research identifies four table-stakes clusters and a set of clear differentiators. JWT auth with tenant isolation is the unconditional prerequisite — no other feature is meaningful without it. Streaming SSE for the chat interface is no longer a nice-to-have but the expected baseline for any LLM chat UX in 2026.

**Must have (table stakes for launch):**
- JWT auth with `tenant_id` claims + tenant registration/email verification — the entire platform collapses without this
- Hard tenant isolation middleware (every MongoDB query includes `tenant_id` filter) — a hard security requirement, not a feature
- BYOK encryption (Fernet + HKDF) for OpenAI keys — required for tenants to use LLM features without platform absorbing unbounded LLM costs
- Per-tenant rate limiting by tier — prevents noisy-neighbor degradation and enables tier monetization
- Streaming SSE conversational chat — users trained by ChatGPT/Claude expect token-by-token output; full-response wait causes abandonment
- Estimate trigger recognition — the system must know when sufficient project detail exists to generate an estimate vs. continue scoping
- Shadow DOM widget with branding config (no "Powered by efOfX" visible) — white-label means the contractor's brand, full stop
- Single-script embed (under 5 lines) — contractors are not developers; complex integration means no adoption
- Lead capture in widget flow — the business justification for contractors to embed the widget
- Magic link customer feedback — token-based, 72-hour expiry, single-use, no customer login required
- Variance calculation and calibration metrics display — even at low data volumes, contractors need to see the system is measuring accuracy

**Should have (competitive differentiators):**
- BYOK as explicit feature proposition — competitors host keys themselves, creating cost liability; tenants controlling their own LLM costs is a real differentiator
- P50/P80 probabilistic ranges (not single numbers) — breaks industry norm of false-precision quotes
- Git-based prompt versioning — prompt changes are auditable, rollbackable, and linked to estimate accuracy outcomes
- Per-tenant reference class accuracy tracking — contractors can demonstrate historical accuracy to their customers
- Contractor feedback form (structured) — customer-reported actuals alone are unreliable signal

**Defer to v1.x (after 5+ active tenants):**
- Contractor-facing calibration trend charts — not meaningful until sufficient data volume
- Automated synthetic data tuning from real outcomes — requires ~100 real feedback submissions for statistical signal
- Audit log query interface — useful for enterprise tenants, not needed for initial contractor base
- Widget embed analytics

**Defer to v2+ (after product-market fit):**
- Communication coaching UI (internal/external narrative toggle) — architecture supports it now; UX complexity is high
- Multiple LLM providers (Anthropic, Gemini) — BYOK solves cost concern; add provider abstraction post-MVP
- SSO (SAML/OAuth) for enterprise tenants — contractor SMB target does not require this
- Automated RLHF fine-tuning — premature without labeled dataset volume and training infrastructure

**Anti-features to explicitly avoid:** Multi-user real-time collaboration, customer accounts and login (magic link is the correct model), global (not per-tenant) rate limits, and LiteLLM multi-provider abstraction at MVP stage.

See `.planning/research/FEATURES.md` for full feature dependency graph and prioritization matrix.

### Architecture Approach

The architecture is brownfield-extension: the FastAPI layered architecture, MongoDB Atlas with Motor, and React widget scaffolding all exist. What is missing are real implementations for six service stubs (`TenantService`, `LLMService`, `ChatService`, `FeedbackService`, plus new `BYOKKeyService` and `CalibrationService`), plus the isolation middleware, prompt registry, and branding config system. The project structure is sound and requires no major reorganization — Epic 7 will clean up the monolithic `routes.py` after implementation is complete.

**Major components and responsibilities:**

1. **TenantIsolationMiddleware** — Extracts and validates `tenant_id` from every JWT at the middleware layer (before any route code executes). Services receive `tenant_id` as their first parameter; they never read request state themselves. This is the enforcement point that makes all tenant isolation reliable regardless of developer discipline.

2. **BYOKKeyService** — Isolated encryption/decryption service. Raw decrypted keys are scoped to the calling function's stack frame; they never appear in logs, object state, or shared memory. Uses HKDF-derived per-tenant keys (not a single master Fernet key) to limit blast radius of a key compromise.

3. **LLMService + PromptRegistry** — `AsyncOpenAI` instantiated per-request with the tenant's decrypted key (never at service init). Prompts loaded from `app/prompts/*.json` at startup and cached in memory; `prompt_version` stored on every EstimationResult for calibration traceability.

4. **ShadowDOMWrapper + WidgetBrandingProvider** — Widget bundled as IIFE with Vite `build.lib`; Shadow DOM isolation via `react-shadow`. Branding config fetched once at `init()` and applied as CSS custom properties on the shadow host — not re-fetched on every chat interaction.

5. **FeedbackService + CalibrationService** — HMAC-signed magic link tokens (not JWT) with 72-hour expiry and single-use enforcement via atomic MongoDB update. Calibration operates exclusively on `data_source: "real"` documents — synthetic data provides prior distribution only.

6. **TenantAwareCollection wrapper** — Every MongoDB `find()`, `find_one()`, `aggregate()`, `update_one()`, and `delete_one()` call goes through a wrapper that auto-injects `tenant_id` filter. This makes cross-tenant queries impossible rather than just discouraged.

**Key patterns:** JWT tenant context propagation via `request.state`; BYOK decrypt-at-call-time; git-versioned JSON prompt files; CSS custom properties for widget branding; atomic magic link single-use enforcement.

See `.planning/research/ARCHITECTURE.md` for full system diagram, data flow sequences, and anti-patterns with code examples.

### Critical Pitfalls

The research identifies six critical pitfalls, two of which are confirmed existing bugs and one of which is a pre-Epic 3 blocker:

1. **Tenant isolation in application code instead of database layer** — The existing `rcf_engine.py` queries reference classes without tenant_id filtering (confirmed bug). Fix: `TenantAwareCollection` wrapper that auto-injects `tenant_id` on all MongoDB operations. Write a cross-tenant test (two tenants; assert tenant A cannot see tenant B's data) and make it pass before closing Epic 3.

2. **LLM parsing stub ships to production** — `_parse_estimation_response()` in `llm_service.py` is hardcoded with default values (confirmed bug). Tenants pay OpenAI API costs (on their BYOK key) for zero-value responses. Fix: OpenAI JSON mode + Pydantic response model. Must be resolved as the first story in Epic 4.

3. **Fernet single master key exposes all tenant OpenAI keys on compromise** — Use per-tenant HKDF-derived keys from day one, not a shared master Fernet key. Add `key_version` field to tenant document for future rotation. Implement startup validation that fails fast if `ENCRYPTION_KEY` is missing.

4. **`DB_COLLECTIONS` import missing in `security.py`** — Existing `NameError` causes 500 errors on any API key auth attempt. This is a pre-Epic 3 fix that must happen immediately.

5. **Shadow DOM does not isolate JavaScript** — CSS/DOM isolation only; JS runs in the same global scope as the host page. Widget must wrap all initialization in `try/catch`; bundle all dependencies into the IIFE; test against adversarial host environments (jQuery 1.x, broken `fetch`, CSP-restricted pages).

6. **Synthetic training data contaminating the calibration loop** — Confirmed by ICLR 2025 research on model collapse. Tag all existing synthetic reference classes with `data_source: "synthetic"` before Epic 6 begins. Calibration calculations must filter exclusively to `data_source: "real"`.

**Additional pitfalls to track:** Async context bleed via module-level globals (replace rate limiter dictionary with Valkey-backed solution); CORS wildcard in production (set explicit origin whitelist with startup warning); widget embed URL versioning (never break existing contractor embeds by changing the script interface).

See `.planning/research/PITFALLS.md` for full recovery strategies, "Looks Done But Isn't" checklist, and integration gotchas.

---

## Implications for Roadmap

The research is unambiguous about phase ordering. JWT auth with tenant isolation is not one of five parallel workstreams — it is the prerequisite for all others. Building BYOK, chat, or the widget before the isolation layer means retrofitting security into every subsequent implementation. The architecture's build order implications directly map to an epic ordering that is already validated by the existing project structure.

### Phase 1: Multi-Tenant Foundation (Epic 3)

**Rationale:** JWT auth with `tenant_id` propagation is the keystone dependency for every other feature. The rate limiter, BYOK encryption, and CORS configuration must all exist before LLM integration can be tenant-aware. Starting elsewhere creates security debt that compounds with every subsequent implementation. Pre-work: fix the `DB_COLLECTIONS` NameError in `security.py` and upgrade `python-jose` → `PyJWT`, `passlib` → `pwdlib`.

**Delivers:** Working multi-tenant platform where tenants can register, authenticate, and have their data isolated from each other with no cross-tenant leakage. Tenant API keys are encrypted at rest with per-tenant HKDF derivation.

**Addresses:** JWT auth, BYOK encryption storage, tenant isolation middleware, per-tenant rate limiting, CORS origin whitelist, audit logging skeleton.

**Avoids:** Cross-tenant data leakage (implement TenantAwareCollection wrapper), async context bleed (ContextVar for tenant identity, no globals), single Fernet master key vulnerability (use HKDF from day one), CORS wildcard in production.

**Research flag:** Standard well-documented patterns (FastAPI middleware, PyJWT, Fernet). No additional phase research needed. Write cross-tenant isolation tests early — they are the acceptance criteria for this phase.

### Phase 2: LLM Integration (Epic 4)

**Rationale:** BYOK key service from Phase 1 is required before LLM calls can be tenant-aware. The LLM parsing stub must be replaced as the first story in this phase — not at the end — because every subsequent story builds on the assumption that `generate_response()` returns real parsed values. Prompt registry can be built in parallel with BYOK refactor since it has no Phase 1 dependencies.

**Delivers:** Working conversational scoping chat with streaming SSE responses, real LLM narrative generation (not stubs), and git-versioned prompt management. Every estimate records `prompt_version` for calibration traceability.

**Uses:** `openai>=2.20.0,<3.0.0` with per-request BYOK key injection, Jinja2/JSON prompt templates, `valkey` for LLM response hash-based caching (SHA-256 cache key, 24h TTL).

**Implements:** LLMService BYOK refactor, PromptRegistry, ChatService with multi-turn context, streaming SSE endpoint.

**Avoids:** LLM parsing stub shipping (replace as story 1), inline prompt strings (all prompts in `app/prompts/*.json`), global OpenAI client instantiation, prompt injection via widget user input (validate LLM output schema, prefix user input with role separator).

**Research flag:** Verify `openai` v1→v2 migration scope in `llm_service.py` before starting. The CHANGELOG has breaking changes in `output` field types. Audit existing service stubs for v1 API usage.

### Phase 3: White-Label Widget (Epic 5)

**Rationale:** The widget is the distribution mechanism — but it requires the chat streaming endpoint from Phase 2 and the tenant branding model (part of tenant registration) from Phase 1. The Shadow DOM scaffolding already exists; new work is the branding config API, BrandingProvider component, and widget IIFE bundling for CDN distribution. Frontend and backend work in this phase can proceed in parallel once the branding API contract is defined.

**Delivers:** Embeddable single-script widget with Shadow DOM CSS isolation, per-tenant branding (colors, logo, welcome message), multi-turn chat UI, lead capture form, and P50/P80 estimate results display. Hosted on DigitalOcean Spaces CDN at versioned URL.

**Uses:** Vite `build.lib` IIFE mode, `react-shadow==20.6.0`, CSS custom properties for branding, `?inline` CSS import for Shadow DOM style injection.

**Implements:** ShadowDOMWrapper extension, WidgetBrandingProvider, ChatInterface, LeadCaptureForm, EstimateDisplay, widget IIFE bundle + CDN deploy pipeline.

**Avoids:** Widget breaking host page on exception (global try/catch around all init), CSS-in-JS inside Shadow DOM (use `?inline` extracted CSS), branding fetch on every chat interaction (fetch once at init, cache in React context), widget embed URL breaking existing contractor sites (version embed URL from day one: `/widget/v1/embed.js`).

**Research flag:** Verify `react-shadow==20.6.0` compatibility with React 19.2.0 before starting (published ~1 year ago; check GitHub issues). Alternative is manual `useEffect` + `attachShadow()`. Write adversarial host page test matrix (jQuery 1.x, broken `fetch`, strict CSP).

### Phase 4: Feedback and Calibration Loop (Epic 6)

**Rationale:** Feedback collection requires completed estimates (Phases 2+3) and tenant email capability (Phase 1). This is the self-improvement moat — the architecture is sound but real calibration value requires data volume. Phase 4 should be shipped promptly to start accumulating real outcome data, even though the calibration metrics will show near-empty values initially.

**Delivers:** Magic link email delivery for customer feedback, contractor feedback form with structured discrepancy fields, actual vs. estimated variance calculation, and tenant calibration dashboard showing mean variance and accuracy trend. Data source tagging (`synthetic` vs. `real`) must be applied retroactively to all existing reference classes before calibration calculations begin.

**Uses:** `fastapi-mail==1.6.2` for magic link delivery, HMAC-signed tokens (not JWT) with 72-hour expiry, atomic MongoDB single-use enforcement, MongoDB aggregation pipeline for calibration metrics.

**Implements:** FeedbackService (magic link generation + outcome capture), CalibrationService (variance calculation, `data_source: "real"` filtering), tenant calibration dashboard endpoint, contractor feedback form.

**Avoids:** Magic link single-use not enforced (atomic MongoDB update with `returnDocument: AFTER`), synthetic data contaminating calibration (filter to `data_source: "real"` exclusively), calibration dashboard showing misleading metrics before minimum sample threshold (show "N more projects needed" below ~20 real outcomes), platform OpenAI key fallback when BYOK fails (return explicit error; never absorb tenant LLM costs).

**Research flag:** The data source migration (tagging existing synthetic reference classes) is a prerequisite migration that must be planned and executed before Epic 6 development begins. Include this in the first story of Phase 4.

### Phase 5: Code Quality and Hardening (Epic 7)

**Rationale:** With all four capability pillars working, Epic 7 addresses the accumulated shortcuts from the rapid implementation phase. The CONCERNS.md audit identified 55+ broad `except Exception` catch-alls, the global in-memory rate limiter that must migrate to Valkey, and route organization that needs splitting from monolithic `routes.py` into domain-separated modules. This phase cannot be done before the features it is hardening.

**Delivers:** Exception hierarchy replacing broad catch-alls, routes split into domain modules, Valkey-backed rate limiting replacing in-memory dictionary, MongoDB connection pool configuration, performance profiling of reference class matching, and cleanup of service stubs that were never properly implemented (YAGNI pass).

**Avoids:** Global rate limiter memory leak (unbounded dictionary growth, OOM risk under load), catch-all exception masking (distinguish OpenAI rate limits from DB failures from validation errors), service instantiation per-request (move to app-lifespan singletons).

**Research flag:** Standard refactoring patterns — no additional phase research needed. Primary input is the CONCERNS.md audit and runtime performance data collected during Phases 1-4.

### Phase Ordering Rationale

- **Phases 1→2→3→4 are a hard dependency chain:** Tenant isolation must exist before BYOK, BYOK must exist before LLM integration, LLM integration must exist before the chat widget has a backend, and the widget must produce estimates before feedback collection is meaningful.
- **Phase 5 (hardening) is correctly last:** Hardening targets identified during Phase 1-4 implementation. Doing it earlier means hardening code that will be rewritten anyway.
- **The dependency chain does not mean sequential sprints:** Within each phase, backend and frontend work can often proceed in parallel once the API contract is defined. Phase 3 especially has parallel tracks.
- **Two immediate pre-work items do not belong in any phase:** Fix the `DB_COLLECTIONS` NameError (pre-Epic 3 blocker) and upgrade the four abandoned/outdated packages (`python-jose`, `passlib`, `openai`, `redis`). These should be tickets that close before Phase 1 begins.

### Research Flags

**Needs validation before starting (but no full research-phase required):**
- **Phase 2:** Audit `llm_service.py` for OpenAI v1→v2 API breaking changes before writing any new LLM code. The CHANGELOG is the source; the audit is an hour of work, not a research sprint.
- **Phase 3:** Verify `react-shadow==20.6.0` works with React 19.2.0. Check the GitHub issues page. If it doesn't, the fallback (manual `useEffect` + `attachShadow()`) is well-documented and requires no additional research.
- **Phase 3:** Verify DigitalOcean App Platform Python 3.11 support (required for `pwdlib` and `fastapi-mail`). Check DO docs or create a test app component before Phase 1 begins.

**Standard patterns (skip research-phase):**
- **Phase 1:** JWT auth, FastAPI middleware, Fernet encryption — all well-documented with official sources verified.
- **Phase 4:** Magic link pattern, HMAC token signing, MongoDB aggregation — straightforward and covered in ARCHITECTURE.md with code examples.
- **Phase 5:** Exception hierarchy, route organization, connection pool tuning — standard Python/FastAPI patterns with no ambiguity.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Core technology choices verified against official docs and PyPI. Package abandonment/deprecation status confirmed with FastAPI community discussions and DigitalOcean official announcements. One MEDIUM-confidence item: `react-shadow` React 19 compatibility needs verification before Phase 3. |
| Features | MEDIUM | Platform architecture (JWT, BYOK, Shadow DOM, magic links) is high-confidence from PRD and codebase. Market expectations for construction contractor UX are LOW-confidence — thin market research, limited to public competitor analysis. Anti-features are well-reasoned and consistent with established patterns. |
| Architecture | HIGH | Based on direct codebase review (existing stubs, models, and service structure confirmed) and official MongoDB, FastAPI, and Fernet documentation. Integration patterns have code examples that align with official SDK documentation. |
| Pitfalls | MEDIUM-HIGH | Six critical pitfalls all verified with multiple sources. Two are confirmed by direct codebase audit (CONCERNS.md). Synthetic data contamination finding backed by ICLR 2025 peer-reviewed research. One LOW-confidence area: Shadow DOM security characteristics drawn from a single source. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **React 19 + react-shadow compatibility:** Unverified. Check GitHub issues for `react-shadow` before Phase 3 begins. Low-risk gap — the fallback path is known.
- **DigitalOcean Python 3.11 App Platform support:** Unverified. `pwdlib` and `fastapi-mail` both require Python 3.10+; current `requirements.txt` may specify 3.9. Verify before Phase 1 begins; this blocks the dependency upgrades.
- **openai v1→v2 migration scope:** The `llm_service.py` stub uses v1 patterns. Full audit needed before Phase 2. Not a research gap — just a code audit task.
- **Construction contractor market expectations:** Feature research for the contractor SMB market was thin. The PRD is the authoritative source here. If tenant feedback during Phase 3-4 reveals unexpected feature expectations, be prepared to reprioritize v1.x features.
- **Valkey SSL/TLS configuration:** DO Managed Valkey requires SSL (`valkeys://` URI). Verify `valkey-py 6.1.0` handles this identically to `redis-py`'s `rediss://` before Phase 1 Valkey integration.

---

## Sources

### Primary (HIGH confidence)
- `docs/PRD.md` — authoritative product requirements source
- `.planning/PROJECT.md` — validated project requirements and constraints
- `docs/architecture.md` — confirmed architectural decisions
- CONCERNS.md codebase audit (2026-02-26) — direct code analysis confirming existing bugs
- PyJWT 2.11.0 — [PyPI](https://pypi.org/project/PyJWT/) — verified 2026-02-26
- openai 2.24.0 — [GitHub releases](https://github.com/openai/openai-python/releases) — verified 2026-02-26
- Fernet encryption — [cryptography 47.0 official docs](https://cryptography.io/en/latest/fernet/)
- MongoDB multi-tenant architecture — [MongoDB Atlas official docs](https://www.mongodb.com/docs/atlas/build-multi-tenant-arch/)
- Vite build lib mode — [Vite build options docs](https://vite.dev/config/build-options)
- OWASP LLM01:2025 Prompt Injection — [OWASP Gen AI Security Project](https://genai.owasp.org/llmrisk/llm01-prompt-injection/)
- AWS SaaS Lens — Identity and Access Management — [AWS official docs](https://docs.aws.amazon.com/wellarchirected/latest/saas-lens/identity-and-access-management.html)
- fastapi-mail 1.6.2 — [PyPI](https://pypi.org/project/fastapi-mail/) — verified 2026-02-26
- cryptography 46.0.5 — [PyPI](https://pypi.org/project/cryptography/) — verified 2026-02-26
- prometheus-fastapi-instrumentator — [GitHub](https://github.com/trallnag/prometheus-fastapi-instrumentator)

### Secondary (MEDIUM confidence)
- python-jose abandonment — [FastAPI discussion #11345](https://github.com/fastapi/fastapi/discussions/11345)
- passlib abandonment — [FastAPI discussion #11773](https://github.com/fastapi/fastapi/discussions/11773)
- DigitalOcean Valkey migration — [DO blog](https://www.digitalocean.com/blog/introducing-managed-valkey)
- WorkOS Developer Guide to Multi-Tenant Architecture — https://workos.com/blog/developers-guide-saas-multi-tenant-architecture
- Frontegg Multi-tenancy Security Best Practices — https://frontegg.com/blog/saas-multitenancy
- JWT Vulnerabilities 2026 — [Red Sentry](https://redsentry.com/resources/blog/jwt-vulnerabilities-list-2026-security-risks-mitigation-guide)
- IronCore Labs: 5 Things SaaS Companies Get Wrong with BYOK — https://ironcorelabs.com/blog/2024/five-things-saas-mess-up-with-byok/
- MakerKit Embeddable Widgets production guide — https://makerkit.dev/blog/tutorials/embeddable-widgets-react
- Buildxact competitor features — https://www.buildxact.com/us/features/construction-estimating-software/
- Streaming LLM responses guide — dev.to and Microsoft Tech Community (multiple sources agree)

### Tertiary (LOW confidence)
- Construction contractor UX expectations — limited market research; PRD is authoritative fallback
- react-shadow React 19 compatibility — unpublished; requires verification before Phase 3
- Shadow DOM security characteristics — single source (CyberSGuards); MDN confirms browser support

### Research (ICLR/Academic)
- Strong Model Collapse — [ICLR 2025](https://proceedings.iclr.cc/paper_files/paper/2025/file/284afdc2309f9667d2d4fb9290235b0c-Paper-Conference.pdf) — confirms synthetic data contamination risk
- Reference Class Forecasting: Problems and Research Agenda — [Taylor & Francis (2025)](https://www.tandfonline.com/doi/full/10.1080/09537287.2025.2578708)
- Fairness Feedback Loops: Training on Synthetic Data Amplifies Bias — [FAccT 2024](https://facctconference.org/static/papers24/facct24-144.pdf)

---
*Research completed: 2026-02-26*
*Ready for roadmap: yes*
