# Feature Research

**Domain:** Feedback loop, calibration dashboard, magic link system, and code quality for multi-tenant estimation SaaS
**Researched:** 2026-02-28
**Confidence:** MEDIUM (core patterns well-understood from authoritative sources; construction-specific calibration expectations remain LOW confidence — no direct precedent found)

---

## Context: v1.1 Milestone Scope

v1.0 is shipped. This research covers only what is NEW in v1.1:

1. **Customer feedback via email magic links** — collect actual project outcomes from customers post-estimate
2. **Contractor calibration dashboard** — aggregate accuracy metrics per tenant
3. **Feedback data collection for manual tuning** — prompt/weight refinement from real patterns
4. **Shared backend/frontend extraction** — vertical reuse prep for IT/dev domain
5. **Tech debt cleanup** — 5 known items from v1.0 audit (INT-04, INT-05, per-process cache, deprecated accessors, ConsultationCTA)

Existing infrastructure: JWT auth, tenant isolation, MongoDB Atlas, FastAPI backend, React widget, OpenAI BYOK, SSE streaming, RCF engine.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features the v1.1 milestone cannot ship without. Missing = feature is broken or unusable.

#### Magic Link Feedback System

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| One-time-use token (cryptographically random) | Replay attack prevention is a hard security requirement; database-level atomic invalidation required | LOW | Generate with `secrets.token_urlsafe(32)` (Python stdlib). Store hashed (SHA-256); compare hash on access. Do NOT store raw token. Atomic mark-used on first successful access prevents race conditions (HIGH confidence — multiple authoritative sources) |
| Short TTL enforcement (72 hours for feedback) | Token must expire; feedback links need longer window than auth links (24-hour project completion cycles) | LOW | Auth magic links: 10-15 min. Feedback collection: 48-72 hours is appropriate — customers need time to receive final invoice. Strictly enforced server-side, not just checked (MEDIUM confidence — Baytech, WorkOS sources) |
| Email with clear CTA and estimate summary | Customer must recognize why they received the link and feel motivated to click | LOW | Email must include: estimate range reminder, simple question ("Did the project come in near this range?"), prominent magic link button. Without context, open rates drop significantly (MEDIUM confidence — NPS email benchmarks show 6-25% response rate, context improves this) |
| Feedback form — actual outcome fields | The calibration engine needs structured data, not free text | LOW | Required fields: actual_cost (number), actual_timeline (dates or duration), optional: rating (1-5), comments. Pre-fill estimate values for comparison. Contractor-side form adds: discrepancy reason (scope_creep, market_change, customer_change, other) |
| Idempotent token validation endpoint | Links are clicked multiple times by accident, forwarded, or pre-fetched by email clients | LOW | Token validation must be: (1) idempotent on re-reads before submission, (2) single-use only on write (form submit). Return 410 GONE on already-used tokens with friendly "already submitted" message (MEDIUM confidence — email client pre-fetch is a documented issue with auth magic links; applies equally here) |
| Graceful expired/invalid token UX | Bad links should not show stack traces or confusing errors | LOW | 3 states: valid (show form), expired (show "link expired, contact contractor"), used (show "feedback already submitted, thank you"). All states return meaningful messages, no 500s |

#### Calibration Dashboard (Contractor-Facing)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Mean variance display (actual vs. estimated) | Core calibration metric — "Your estimates are trending X% high/low" | LOW | variance = (actual - estimated) / estimated. Display as signed percentage. Mean variance over all submissions with feedback. Negative = trending low (underestimates), positive = trending high (MEDIUM confidence — Amazon Forecast docs, statistical calibration literature) |
| Count metrics (estimates sent, feedback received, feedback rate) | Contractors need to know if the feedback sample is statistically meaningful | LOW | Show: total estimates, feedback received, feedback rate (%). Low sample sizes should be clearly flagged — "3 submissions is not statistically significant" (MEDIUM confidence — calibration literature) |
| Accuracy bucket ("% within 20% of actual") | The PRD's stated success metric; what contractors can quote to customers | MEDIUM | Bin submissions: within 10%, within 20%, within 30%, >30% off. Display as counts and percentages. 70% within 20% is the PRD target. This is what contractors use to say "our estimates are historically accurate within X%" (HIGH confidence — PRD requirement, maps to standard forecast accuracy reporting) |
| Per-reference-class accuracy breakdown | Different project types have different accuracy profiles; a roof job is not a pool job | MEDIUM | Group feedback by reference_class_id or category. Allow contractors to see which project types their estimates nail vs. miss. Useful for prompt/weight tuning decisions (MEDIUM confidence — pattern from demand forecasting dashboards) |
| Temporal trend (rolling accuracy over time) | One bad period should not define accuracy; contractors want to see improvement | MEDIUM | Rolling 30/90-day mean variance. Trend direction (improving/declining/stable). Display only when enough data exists (>5 submissions per window) (MEDIUM confidence — forecast accuracy dashboard patterns) |

#### Feedback Data Collection for Manual Tuning

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Feedback stored against estimate snapshot | Tuning requires knowing what the estimate WAS, not just what reality turned out to be | LOW | Store feedback document with: estimate_id, estimate_snapshot (P50, P80, reference_class), actual_cost, actual_timeline, variance_pct, submitted_at. Do NOT overwrite the original estimate; the snapshot is immutable (MEDIUM confidence — data integrity requirement) |
| Contractor discrepancy reason (structured enum) | Free text is not actionable for tuning; structured reasons enable pattern detection | LOW | Enum values: scope_creep, market_change, customer_change, estimate_error, unexpected_conditions, other. Multiple allowed. This drives the "what to adjust" decision in manual tuning (MEDIUM confidence — derived from PRD feedback form spec) |
| Admin-facing raw feedback list (not per-tenant) | Manual tuning requires seeing feedback across tenants to detect cross-cutting patterns | LOW | Simple table: date, tenant, reference_class, variance_pct, discrepancy_reason, comments. Not a fancy dashboard — enough for a human reviewer to identify patterns. Tenant names visible to admin only (MEDIUM confidence — manual human-in-loop process) |
| Feedback → reference class linkage | Tuning synthetic data requires knowing which reference class was matched for each estimate | LOW | Persist the matched_reference_class_id in the estimate document at creation time. Feedback links back to this. Required for "reference class X is consistently overestimating by 15%" analysis (LOW confidence — inferred from RCF calibration needs, not externally verified) |

#### Tech Debt Items (Non-Negotiable Cleanup)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Fix EstimationSession tenant_id type (INT-04) | Type mismatch causes query failures or silent tenant isolation bugs | LOW | Confirm field type matches TenantAwareCollection expectations. MongoDB ObjectId vs. string — must be consistent across all documents and query paths |
| Add missing widget indexes (INT-05) | Missing indexes cause slow widget analytics queries at scale (100k estimates/month target) | LOW | Add compound indexes for widget_analytics collection: (tenant_id, event_type, created_at). Measure query plans before and after |
| Replace per-process LLM cache with Valkey | Per-process cache breaks on multi-worker deployments; cache misses 100% of the time when request hits a different worker | MEDIUM | Valkey is a drop-in Redis-compatible replacement (BSD license, DigitalOcean Managed Databases offers managed Valkey). Use fastapi-cache2 with Valkey backend. Content-hash key strategy is already correct — just change the backend (HIGH confidence — Valkey official docs, DigitalOcean product page, fastapi-cache2 PyPI page) |
| Wire ConsultationCTA button destination | Dead UI element damages user trust; an unlinked button is a broken experience | LOW | Confirm destination URL (contractor contact flow or external link). One-line fix but requires product decision on destination |
| Remove deprecated collection accessors | Stale accessor patterns cause maintenance confusion and hide tenant isolation bugs | LOW | Dead code removal: identify deprecated accessors, verify no call sites remain (use static analysis / grep), delete. Write test confirming isolation still holds after removal |

---

### Differentiators (Competitive Advantage)

Features that make the v1.1 feedback loop genuinely better than generic survey approaches.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Estimate-contextualized feedback form | Pre-populating the form with the original estimate range makes actual vs. estimated comparison effortless; cold surveys get 6-12% response rates vs. contextualized ones getting closer to 25% | LOW | Include in the feedback email and form: "We estimated $X-$Y. How did the project actually come in?" Simple anchoring dramatically improves response quality (MEDIUM confidence — survey response rate research) |
| Contractor-can-see-customer-feedback workflow | Contractors often know why an estimate was off (scope creep, customer changes) but customers blame "bad estimate"; dual-perspective data is the signal | LOW | After customer submits, trigger (optional) contractor notification: "Customer reported $X actual vs your $Y estimate — any context to add?" Contractor sees customer's answer, can add explanation. Creates clean two-sided signal (LOW confidence — inferred from PRD requirements; no external precedent verified) |
| Reference class level accuracy labels | Contractors can use per-category accuracy as a sales tool ("our kitchen remodels are accurate within 15%") — not just global accuracy | MEDIUM | Labels derived from calibration data per reference class. Only show when N >= 5 submissions. These are the "trust signals" from the PRD success criteria (MEDIUM confidence — PRD requirement) |
| Shared util/component library with clear domain boundaries | If shared libraries are extracted correctly, the IT/dev vertical is a configuration change, not a rewrite | MEDIUM | Backend: extract tenant middleware, auth utilities, base document models, RCF matching interface into a shared package. Frontend: extract design system tokens, chat widget shell, lead capture form, estimate display component into a shared library. Domain-specific code stays in vertical apps (MEDIUM confidence — monorepo best practices) |

---

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem like obvious additions but are wrong for v1.1 scope.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Automated LLM prompt tuning from feedback | "The system should automatically improve based on feedback" | Automated prompt updates without human review is a reliability disaster. A bad feedback signal (outlier project with scope creep) could degrade prompts for all estimates. LLM output quality is hard to regression-test automatically at this scale | Manual pattern review: admin sees feedback patterns, human decides what to change in the prompt, changes versioned via git. Automate in v2 when you have labeled evaluation sets |
| Automated reference class splitting/merging | "When a reference class is consistently off, it should automatically refactor" | Requires statistical confidence intervals, sufficient sample sizes, and validation that new classes perform better. At <100 feedback submissions, this is noise chasing | Manual review of calibration reports. When variance in a class is consistently >20%, flag for human review. Automated splitting is a v2+ feature |
| Email drip campaign after feedback non-response | "We should follow up if they don't click the link" | Customers who ignore the first email are signaling disinterest. Drip campaigns on feedback links annoy contractors' customers (your tenant's relationship is at stake). Re-engagement emails require GDPR/CAN-SPAM compliance work | Single send only. Optional: contractor can manually resend once via dashboard. Do not automate follow-up |
| Customer accounts (login) for feedback history | "Customers should be able to log in and see all their estimates" | Customer auth doubles the auth surface (tenant auth + customer auth), requires password reset, email verification, and a customer-facing UX. Customers are one-time quote requestors, not repeat users | Magic link for each estimate. Stateless feedback. No customer accounts in v1.x |
| Public-facing accuracy statistics page | "Our accuracy track record should be publicly visible" | Exposes calibration data that could undermine contractor client relationships (if a contractor's estimates are consistently 30% off). Also requires privacy review — feedback submissions are between contractor and customer | Per-tenant accuracy visible only to the tenant. Public accuracy claims belong to marketing copy, not a live dashboard |
| Full component Storybook documentation | "Shared components should be documented" | Storybook setup and maintenance is a meaningful ongoing investment. For a 2-vertical extraction, it's over-engineered at this stage | TypeScript types + JSDoc + usage examples in the README. Storybook when there are 3+ verticals consuming shared components |
| Shared component npm publish to registry | "Other teams/projects might want to use these components" | For an internal monorepo, npm publish adds versioning, changelog, and release management overhead that slows iteration. No external consumers exist yet | pnpm workspace protocol (`workspace:*`) for internal monorepo sharing. Evaluate registry publish when external consumers emerge |

---

## Feature Dependencies

```
[v1.0 Estimate Results]
    └──triggers──> [Feedback Email Send]
                       └──requires──> [Magic Link Token Generation]
                       └──requires──> [Email Delivery (existing or new)]

[Magic Link Token Generation]
    └──requires──> [estimate_id reference]
    └──requires──> [token store (MongoDB tokens collection)]
    └──produces──> [signed URL with token param]

[Feedback Email Send]
    └──requires──> [Customer email (from lead capture — already collected)]
    └──requires──> [Magic Link Token Generation]
    └──enables──> [Customer Feedback Form]

[Customer Feedback Form]
    └──requires──> [Token validation (one-time, TTL-checked)]
    └──requires──> [Estimate snapshot (pre-populated)]
    └──enables──> [Feedback Document Write]

[Contractor Feedback Notification]
    └──requires──> [Customer Feedback Document]
    └──enhances──> [Feedback Document] (adds contractor context)

[Feedback Document Write]
    └──requires──> [estimate_id, reference_class_id, actual_cost, actual_timeline]
    └──enables──> [Calibration Metrics Calculation]

[Calibration Metrics Calculation]
    └──requires──> [Feedback Document(s)]
    └──produces──> [mean_variance, accuracy_buckets, per_class_breakdown]
    └──enables──> [Calibration Dashboard Display]
    └──enables──> [Manual Tuning Decision (human)]

[Calibration Dashboard Display]
    └──requires──> [Calibration Metrics Calculation]
    └──requires──> [JWT auth (contractor login — already exists)]

[Manual Tuning Decision]
    └──requires──> [Admin feedback review interface]
    └──enables──> [Prompt update (git commit to prompt file)]
    └──enables──> [Synthetic data weight adjustment]

[Shared Library Extraction]
    └──requires──> [Inventory of current shared code]
    └──produces──> [shared/backend/ package, shared/frontend/ package]
    └──enables──> [IT/dev vertical (next milestone)]

[Tech Debt Cleanup]
    └──INT-04 fix ──requires──> [type audit of EstimationSession]
    └──INT-05 fix ──requires──> [MongoDB index plan review]
    └──Valkey migration ──requires──> [Valkey instance provisioned on DigitalOcean]
    └──ConsultationCTA ──requires──> [product decision on destination URL]
    └──Deprecated accessors ──requires──> [call-site audit]
```

### Dependency Notes

- **Magic link requires customer email already collected:** Lead capture form in v1.0 widget already collects customer email — the feedback email send does not need to ask for it again. HIGH confidence this is available; verify field name in the lead document schema before implementing.
- **Feedback form is stateless:** No session, no customer login. Token is the only identity. This simplifies implementation significantly — a single-page React form at a public route, validated against the token endpoint.
- **Calibration dashboard is read-only for contractors:** Contractors see aggregated numbers, not individual customer responses. This is a privacy and trust boundary — contractors should not see raw customer comments by default (customers may be candid about the contractor's quality of work).
- **Valkey must be provisioned before code change:** DigitalOcean Managed Databases supports Valkey. Provision first, then update the FastAPI cache backend. Running without Valkey during transition causes 100% cache misses — acceptable for the transition window, not for production.
- **Shared library extraction does not block any v1.1 user-facing feature:** It is a refactoring milestone. Can be sequenced after feedback/calibration features are shipped. Sequencing it last reduces risk of breaking changes mid-milestone.

---

## MVP Definition for v1.1

### Ship in v1.1 (Required)

- [ ] **Magic link token generation + email send** — The feedback loop cannot start without this; customers need a frictionless path to submit outcomes
- [ ] **Feedback form (customer + contractor)** — Structured data collection; free text alone is not tunable
- [ ] **Token validation endpoint** — Idempotent read, single-use write, proper expiry and error states
- [ ] **Feedback document storage** — With estimate snapshot, reference class linkage, and discrepancy reason enum
- [ ] **Calibration metrics calculation** — Mean variance, accuracy buckets, per-class breakdown (even with low data, the framework must be present)
- [ ] **Contractor calibration dashboard** — Read-only view of their own calibration data; the reason contractors will trust the system over time
- [ ] **INT-04 fix (tenant_id type)** — Risk of silent tenant isolation bugs; must fix before feedback data accumulates under wrong type
- [ ] **INT-05 fix (widget indexes)** — Performance correctness before scale; widget analytics will be queried in dashboard
- [ ] **Valkey cache migration** — Multi-worker deployments are broken without this; shipping more users onto a broken cache is wrong
- [ ] **ConsultationCTA wiring** — Dead UI element in production; one-line fix with product decision
- [ ] **Deprecated accessor removal** — Code clarity before extraction; easier to extract clean code than to extract around dead code
- [ ] **Shared backend utilities extraction** — Tenant middleware, auth utils, base models extracted to shared package (needed for v1.2 IT/dev vertical)
- [ ] **Shared frontend components extraction** — Widget shell, lead capture, estimate display extracted to shared library
- [ ] **YAGNI pass** — Delete unused code paths before extraction; extracting dead code into shared libs compounds the maintenance burden

### Add After v1.1 Validates (v1.2+)

- [ ] **Automated contractor notification after customer submits** — Useful after feedback volume exists to make it worth wiring
- [ ] **Temporal trend charts on calibration dashboard** — Requires >30 days of data per tenant to be meaningful
- [ ] **Synthetic data weight adjustment workflow** — Requires enough feedback to have statistical confidence per reference class (N>=10 per class)
- [ ] **IT/dev vertical reference classes** — Uses the shared library extraction from v1.1; this is the vertical proving the architecture

### Future Consideration (v2+)

- [ ] **Automated prompt tuning from feedback patterns** — Requires human-validated evaluation harness before automation is safe
- [ ] **Automated reference class splitting/merging** — Requires sufficient feedback volume and statistical rigor
- [ ] **Public accuracy badge/track record page** — Requires privacy review and contractor opt-in

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Magic link token system | HIGH | LOW | P1 |
| Customer feedback form | HIGH | LOW | P1 |
| Feedback document storage | HIGH | LOW | P1 |
| Calibration metrics calculation | HIGH | MEDIUM | P1 |
| Contractor calibration dashboard | HIGH | MEDIUM | P1 |
| INT-04 tenant_id type fix | HIGH (correctness) | LOW | P1 |
| INT-05 widget indexes | MEDIUM | LOW | P1 |
| Valkey cache migration | HIGH (correctness) | MEDIUM | P1 |
| ConsultationCTA wiring | LOW | LOW | P1 |
| Deprecated accessor removal | MEDIUM (debt) | LOW | P1 |
| Shared backend extraction | HIGH (future vertical) | MEDIUM | P1 |
| Shared frontend extraction | HIGH (future vertical) | MEDIUM | P1 |
| YAGNI pass | MEDIUM (debt) | LOW | P1 |
| Contractor feedback notification | MEDIUM | LOW | P2 |
| Temporal calibration trends | MEDIUM | LOW | P2 |
| Admin raw feedback list | LOW | LOW | P2 |
| Synthetic data weight adjustment | HIGH | HIGH | P2 |

**Priority key:**
- P1: Must ship in v1.1
- P2: Add after v1.1 feedback/validation
- P3: Future consideration

---

## Implementation Complexity Notes

### Magic Link System

The token mechanics are straightforward (LOW complexity). The hard parts are:

1. **Email deliverability** — SPF/DKIM/DMARC must be configured. Transactional email provider (SendGrid, Resend, or Postmark) is required. This is the most common failure mode for magic link systems. If email goes to spam, feedback rate is zero regardless of UX quality.
2. **Email client pre-fetch** — Some email security tools (ProofPoint, others) pre-fetch links in emails to check for malware. If the token is consumed on pre-fetch, the customer's click shows "link already used." Mitigation: make the GET to the token URL idempotent (read-only verification); only consume the token on form POST submission.
3. **Token storage in MongoDB** — Store in a dedicated `feedback_tokens` collection with: `token_hash`, `estimate_id`, `tenant_id`, `customer_email`, `expires_at`, `used_at`. TTL index on `expires_at` for automatic expiry cleanup.

### Calibration Dashboard

Complexity depends on data volume assumptions:

- With <50 feedback submissions per tenant: simple aggregation queries (LOW complexity — a MongoDB `$group` pipeline)
- With >1000 submissions per tenant: need pre-computed materialized views or background aggregation jobs (MEDIUM complexity)

For v1.1 at 15 tenants and early feedback volumes, simple aggregation is correct. Flag that this needs revisiting when any tenant exceeds 500 submissions.

### Shared Library Extraction

The risk is in boundary definition, not implementation:

- **Too broad:** Shared library becomes a monolith that changes for vertical-specific reasons. Every vertical is coupled to every other vertical's concerns.
- **Too narrow:** Shared library doesn't actually reduce duplication. Verticals still implement the same patterns independently.

Right boundary: Extract code that is **identical across verticals** (tenant auth middleware, base document models, RCF interface contract, design system tokens, widget shell). Leave domain-specific logic (construction reference classes, IT/dev reference classes, industry-specific prompt templates) in the vertical app.

Pattern: shared/backend is a Python package with explicit public API. shared/frontend is a TypeScript package with explicit index.ts re-exports. Both use workspace protocol, not npm publish.

### Valkey Migration

LOW risk if done correctly:

1. Provision DigitalOcean Managed Valkey instance
2. Add VALKEY_URL to environment config
3. Swap fastapi-cache2 backend from in-memory to Valkey
4. Test: confirm cache hits work across two simultaneous workers
5. Monitor: watch cache hit rate for 24 hours post-deploy

The content-hash cache key strategy already in place is correct for Valkey — no key changes needed.

---

## Sources

- Baytech Consulting — Magic Links UX, Security and Growth (2025): https://www.baytechconsulting.com/blog/magic-links-ux-security-and-growth-impacts-for-saas-platforms-2025 (MEDIUM confidence — consulting analysis, single source)
- WorkOS — Guide to Magic Links: https://workos.com/blog/a-guide-to-magic-links (MEDIUM confidence — practitioner guide)
- Guptadeepak — Magic Link Security Best Practices: https://guptadeepak.com/mastering-magic-link-security-a-deep-dive-for-developers/ (MEDIUM confidence — developer guide)
- Amazon Forecast Docs — Evaluating Predictor Accuracy (P50/P80 metrics): https://docs.aws.amazon.com/forecast/latest/dg/metrics.html (HIGH confidence — official AWS documentation)
- Cultivate Labs — What is Forecast Calibration: https://www.cultivatelabs.com/crowdsourced-forecasting-guide/what-is-forecast-calibration (MEDIUM confidence — practitioner source)
- DNV — Terminology Explained P10, P50, P90: https://www.dnv.com/article/terminology-explained-p10-p50-and-p90-202611/ (HIGH confidence — engineering standards organization)
- DigitalOcean — Managed Caching for Valkey: https://www.digitalocean.com/products/managed-databases-valkey (HIGH confidence — official product page)
- fastapi-cache2 PyPI: https://pypi.org/project/fastapi-cache2/ (HIGH confidence — official package registry)
- Monorepo Tools — Monorepo Explained: https://monorepo.tools/ (HIGH confidence — community reference)
- Feature-Sliced Design — Monorepo Architecture Guide 2025: https://feature-sliced.design/blog/frontend-monorepo-explained (MEDIUM confidence — community guide)
- AskYazi — Survey Response Rates Guide (NPS and Post-Interaction): https://www.askyazi.com/articles/survey-response-rates-a-complete-guide-to-nps-and-post-interaction-feedback (MEDIUM confidence — survey methodology source)
- efOfX PRD: docs/PRD.md (HIGH confidence — authoritative source of record)
- efOfX PROJECT.md: .planning/PROJECT.md (HIGH confidence — confirmed validated requirements)

---

*Feature research for: v1.1 Feedback Loop, Calibration Dashboard, Shared Library Extraction, Tech Debt Cleanup*
*Researched: 2026-02-28*
