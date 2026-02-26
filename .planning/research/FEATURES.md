# Feature Research

**Domain:** Multi-tenant B2B SaaS estimation platform with LLM integration and white-label widgets
**Researched:** 2026-02-26
**Confidence:** MEDIUM (platform architecture confirmed from PROJECT.md + PRD; ecosystem validated against current 2026 sources; specific construction contractor expectations LOW confidence — market research thin)

---

## Context: What Exists (Epics 1-2 Complete)

The RCF engine and foundation are live. This research covers the four capability pillars being added:

1. **Multi-tenant security** — Epic 3: tenant isolation, JWT auth, BYOK encryption
2. **LLM conversational estimation** — Epic 4: chat scoping, narrative generation, prompt management
3. **White-label embeddable widget** — Epic 5: Shadow DOM widget, branding, lead capture
4. **Feedback/calibration loops** — Epic 6: magic link collection, calibration metrics, self-improvement

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features that users assume exist. Missing these = product feels incomplete or insecure.

#### Multi-Tenant Security (B2B SaaS Baseline)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| JWT auth with tenant_id claims | Every B2B SaaS requires stateless auth with tenant context in token | MEDIUM | Claims: sub, tenant_id, role, iat, exp. Authorization decisions must be tenant-scoped, not global (MEDIUM confidence — WorkOS guide, verified against AWS SaaS Lens) |
| Tenant registration + email verification | Users cannot trust unverified tenants in a shared data environment | LOW | Email verification prevents throwaway tenant accounts; required for BYOK activation trust |
| Hard tenant isolation on every query | Zero cross-tenant data leakage is a hard security requirement, not a feature | MEDIUM | Every MongoDB query MUST include tenant_id filter — architectural enforcement, not optional. Missing this causes rewrite (HIGH confidence — multiple authoritative sources) |
| Per-tenant rate limiting by tier | Prevents noisy-neighbor resource exhaustion and enables tier monetization | MEDIUM | Token bucket or sliding window per tenant_id; differentiates free/paid tiers |
| Audit logging per tenant | Enterprise B2B buyers require audit trails; missing = sales blocker | LOW | Log estimation requests, API key usage, config changes; queryable per tenant |
| API key rotation support | Encrypted keys need rotation capability; BYOK tenants manage their own keys | LOW | Allow tenants to update their OpenAI key without re-registration |

#### LLM Conversational Estimation (AI Chat Baseline)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Streaming responses (SSE) | Users trained by ChatGPT/Claude expect token-by-token output; waiting 10s for full response causes abandonment | HIGH | "Streaming isn't a nice-to-have — it's the new standard" (2025). FastAPI supports SSE natively. Missing this makes the chat feel broken (MEDIUM confidence — multiple dev community sources) |
| Conversation history persistence | Users expect chat context to survive page refresh during active session | MEDIUM | Session-scoped; not indefinite history. Store session_id → conversation_history in MongoDB with TTL index |
| Graceful LLM failure handling | API timeouts, rate limits, and key exhaustion WILL happen; unhandled = broken experience | MEDIUM | Fallback messaging, retry with backoff, surface errors clearly to user (not stack traces) |
| Clear "thinking" state indicator | Users abandon if they don't know the system is working | LOW | Loading animation while LLM generates; distinct from streaming state |
| Estimate trigger recognition | Chat must know when enough info exists to generate an estimate vs. keep asking | HIGH | The core scoping intelligence — determine sufficiency from project description |

#### White-Label Widget (Contractor Baseline)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| No "Powered by efOfX" branding visible | White-label means the contractor's brand, not the platform vendor's | LOW | Complete brand suppression; contractors will reject widgets that expose vendor branding to their customers |
| Color + logo customization | Contractor sites have brand standards; a widget that clashes looks unprofessional | MEDIUM | CSS custom properties + logo URL via API config fetch at init time |
| Mobile-responsive layout | Contractor customers use phones to request quotes; non-responsive = unusable | MEDIUM | Widget must work in sidebar, modal, and full-width on mobile |
| Single script embed (<5 lines) | Contractors are not developers; complex integration means no adoption | LOW | `<script src="...embed.js"></script>` + init call. This is the distribution moat |
| Lead capture (email/phone) | Contractors need contact info from prospects; no lead capture = the widget has no business value to them | LOW | Configurable required fields; captured leads stored under tenant account |
| CORS configuration | Widgets embedded on third-party domains require correct CORS headers | LOW | Origin whitelist per tenant; missing = browser blocks all widget API calls |

#### Feedback/Calibration (Self-Improvement Baseline)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Magic link for customer feedback | Customers won't create accounts to submit feedback; magic link is the only viable UX | LOW | Token: cryptographically random, 72-hour expiration, one-time use, hashed storage. Short-lived enough to be secure, long-enough for realistic project completion timelines |
| Variance calculation (actual vs. estimated) | The core promise of calibration requires quantitative accuracy measurement | MEDIUM | (actual - estimated) / estimated per submission; tracked by reference class |
| Tenant-level calibration dashboard | Contractors need to see if their estimates are trending high/low to trust the system | MEDIUM | Aggregate accuracy metrics per tenant: mean variance, % within 20%, trend over time |
| Contractor feedback path | Customer-reported actuals are unreliable alone; contractor explanation of discrepancies is essential for clean signal | LOW | Separate form with structured fields: actual cost breakdown, discrepancy reason (scope creep, market change, etc.) |

---

### Differentiators (Competitive Advantage)

Features that set efOfX apart. These connect directly to the "trust through transparency" competitive moat.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| BYOK (Bring Your Own Key) for OpenAI | Tenants control their own LLM costs and data exposure; platform never touches their billing | MEDIUM | Fernet symmetric encryption for stored keys. Fallback to platform key for trials. Most competitors host keys themselves, creating cost liability (MEDIUM confidence — JetBrains/Warp BYOK implementations reviewed) |
| P50/P80 probabilistic ranges instead of single number | Breaks the industry norm of false-precision quotes; educates contractors to communicate uncertainty | HIGH | The two-layer model (internal truth vs. external narrative) is core to differentiation. Competitors give single numbers; efOfX gives ranges with explanations |
| Self-improving estimates from real outcome data | Every project completion makes the next estimate better; creates compounding accuracy advantage | HIGH | Feedback → calibration metrics → synthetic data tuning → reference class improvement. This is the moat — takes years of competitor use to replicate |
| Per-tenant reference class accuracy tracking | Contractors can show customers their historical track record ("within 15% for 68% of similar projects") | MEDIUM | Trust signal that competitors can't offer without equivalent feedback infrastructure |
| Git-based prompt versioning | Prompt changes are auditable, rollbackable, and linked to estimate accuracy outcomes | MEDIUM | Store prompt_version in every estimate document; enables retrospective accuracy analysis by prompt version. Most platforms hardcode prompts (MEDIUM confidence — prompt management research) |
| Conversational scoping (not a form) | Chat-based project detail gathering feels modern and reduces abandonment vs. long static forms | HIGH | The widget chat is the distribution mechanism AND the UX differentiator. Competitors either use forms or require API integration by contractors |
| Dual audience narratives (contractor vs. customer) | Same estimate data, different communication framing for internal use vs. customer-facing | HIGH | Post-MVP (out of scope for current epics per PRD) but the architecture supports it now |

---

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create scope, security, or maintenance problems for this stage.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Multiple LLM providers (Anthropic, Gemini, etc.) | Appears to reduce vendor lock-in | Requires provider-agnostic abstraction layer, different token pricing, different API contracts, multiplied testing surface. BYOK already solves cost concern — tenants use their own keys | OpenAI BYOK only for MVP. Architecture can add providers post-MVP through abstraction interface |
| Real-time multi-user collaboration on estimates | "Can multiple contractors work on the same estimate?" seems valuable | Requires WebSocket infrastructure, conflict resolution, and operational complexity entirely out of proportion to MVP needs. The estimation workflow is inherently async | Async session model: one user creates, shares via link or email |
| SSO (SAML/OAuth enterprise login) | Enterprise buyers often require SSO | Adds weeks of implementation for a 15-tenant MVP. The tenants are small contractors, not enterprise buyers requiring corporate SSO | JWT + email verification for MVP; add SSO in post-MVP enterprise tier |
| Advanced analytics dashboard for tenants | Tenants will ask for charts and reporting | Feature creep that delays core product. Before you have real feedback data, analytics have nothing meaningful to show | Ship calibration metrics (variance %, trend) only. Full analytics dashboard is post-MVP |
| Global rate limiting rules (not per-tenant) | Simpler to implement one global limit | Penalizes large tenants for small tenants' inactivity; creates unfair throttling and removes tier differentiation capability | Per-tenant rate limits keyed on tenant_id from JWT |
| Widget iframe embedding | Simpler cross-origin isolation | iframes are slow, clunky, and restricted in many security headers. Cannot share fonts, cannot communicate easily with parent page, mobile UX degrades. Shadow DOM with web component is strictly better in 2025 | Shadow DOM with closed mode for security; direct DOM injection with style isolation |
| Automatic LLM fine-tuning from feedback | "The system should learn automatically" | RLHF fine-tuning requires labeled datasets, training infrastructure, and careful calibration validation. Miscalibration post-RLHF is a documented failure mode. Too complex for MVP | Prompt refinement from feedback patterns (human-in-loop); synthetic data tuning; save fine-tuning for post-MVP when you have real outcome volume |
| Communication coaching UI toggle (internal/external) | The two-layer model is the core vision | Genuinely valuable but requires significant UI/UX design, stakeholder role modeling, and educational content. Doing it poorly would undermine the trust narrative | Post-MVP. Architecture already supports dual audiences; defer the UI |
| Customer accounts and login | "Customers should be able to review their estimate history" | Adds auth complexity for end customers who are one-time quote requestors. Increases scope massively | Magic link for feedback access; no persistent customer accounts in MVP |

---

## Feature Dependencies

```
[Tenant Registration] ──requires──> [Email Verification]
                            └──enables──> [JWT Auth with tenant_id]
                                              └──enables──> [All tenant-scoped features]

[JWT Auth with tenant_id]
    └──requires──> [Tenant Isolation Middleware]
                       └──enforces──> [Per-query tenant_id filter]

[BYOK Encryption (Fernet)]
    └──requires──> [Tenant Registration complete]
    └──enables──> [LLM Integration (OpenAI calls)]

[LLM Integration]
    └──requires──> [BYOK Encryption] OR [Platform fallback key]
    └──requires──> [Git-based Prompt Management]
    └──enables──> [Conversational Scoping Chat]
    └──enables──> [Estimate Narrative Generation]

[Conversational Scoping Chat]
    └──requires──> [LLM Integration]
    └──requires──> [Streaming SSE endpoint]
    └──enables──> [Widget Chat UI]

[Widget Chat UI]
    └──requires──> [Conversational Scoping Chat]
    └──requires──> [Shadow DOM Container]
    └──requires──> [Widget Branding Config API]
    └──requires──> [CORS configuration]
    └──enables──> [Lead Capture Form]
    └──enables──> [Estimate Results Display]

[Estimate Results Display]
    └──requires──> [Widget Chat UI]
    └──requires──> [RCF Engine] (already exists, Epic 2)
    └──enables──> [Magic Link Feedback Collection]

[Magic Link Feedback Collection]
    └──requires──> [Estimate Results Display] (needs an estimate to collect against)
    └──requires──> [SendGrid email integration]
    └──enables──> [Calibration Metrics Calculation]

[Calibration Metrics Calculation]
    └──requires──> [Magic Link Feedback] OR [Contractor Feedback]
    └──enables──> [Tenant Calibration Dashboard]
    └──enables──> [Synthetic Data Tuning]
    └──enables──> [LLM Prompt Refinement from Feedback]

[Per-tenant Rate Limiting]
    └──requires──> [JWT Auth with tenant_id]

[Audit Logging]
    └──requires──> [JWT Auth with tenant_id]
```

### Dependency Notes

- **JWT auth is the keystone dependency:** Everything tenant-scoped flows from it. Must be complete before any other Epic 3-6 work.
- **BYOK must precede LLM integration:** The LLM client has no key to use until BYOK storage is implemented. Platform fallback key can unblock parallel development if needed.
- **Widget requires backend chat API:** Shadow DOM container and frontend are irrelevant without the streaming chat endpoint. Build backend Epic 4 before frontend Epic 5.
- **Calibration requires real feedback volume:** The calibration dashboard will show near-empty metrics initially. This is expected — synthetic data validates the schema; real data drives improvement.
- **Magic link conflicts with customer accounts:** These are mutually exclusive approaches to feedback collection. Chose magic link for MVP (confirmed correct by PRD).

---

## MVP Definition

### Launch With (Epics 3-6 Scope)

These are the minimum features needed to validate the efOfX value proposition with real contractors.

- [x] **Tenant registration + JWT auth with tenant_id claims** — Without this, nothing is isolatable; the whole platform collapses
- [x] **BYOK encryption (Fernet) for OpenAI keys** — Required for tenants to use LLM features; platform liability without it
- [x] **Hard tenant isolation middleware** — Zero cross-tenant data leakage is a hard security requirement
- [x] **Per-tenant rate limiting** — Prevents one tenant from degrading others; required for multi-tenancy to be viable
- [x] **Conversational scoping chat with streaming** — The core user experience of the platform; drives widget adoption
- [x] **LLM narrative generation** — Without this, estimates are just numbers; the narrative is what educates contractors
- [x] **Git-based prompt versioning** — Required to safely iterate on prompts post-launch without breaking accuracy
- [x] **Shadow DOM widget with branding config** — The distribution mechanism; without this, no contractors embed the widget
- [x] **Lead capture in widget flow** — The business justification for contractors to embed the widget
- [x] **Magic link customer feedback** — Required to close the feedback loop; the self-improvement moat starts here
- [x] **Contractor feedback form** — Customer data alone is insufficient signal; contractor context is required for clean calibration
- [x] **Variance calculation and calibration metrics** — Even with minimal data, contractors need to see the system is measuring accuracy

### Add After Validation (v1.x)

Add these once 5+ contractors are actively using the system.

- [ ] **Contractor-facing calibration trend charts** — Requires enough data to be meaningful; not useful day 1
- [ ] **Automated synthetic data tuning from real outcomes** — Requires ~100 real feedback submissions to have statistical signal
- [ ] **LLM prompt refinement from feedback patterns** — Requires human review of patterns before automating; don't automate blind
- [ ] **Audit log query interface** — Useful for enterprise tenants; not needed for initial small contractor base
- [ ] **Widget embed analytics (views, starts, completions)** — Nice tracking but not core to the value prop at launch

### Future Consideration (v2+)

Defer until product-market fit is established.

- [ ] **Communication coaching UI (internal/external toggle)** — The vision feature; architecture supports it but UX complexity is high
- [ ] **IT/development domain reference classes** — Proves domain-agnostic architecture; add after construction domain validated
- [ ] **SSO (SAML/OAuth) for enterprise tenants** — Enterprise sales motion; not needed for contractor SMB target
- [ ] **Multiple LLM providers** — Add Anthropic/Gemini abstraction post-MVP; BYOK solves the cost concern for now
- [ ] **Automated RLHF fine-tuning** — Requires labeled dataset volume and training infrastructure; premature for MVP
- [ ] **Marketplace for anonymized reference class sharing** — Community feature that requires substantial tenant base to be valuable

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| JWT auth + tenant isolation | HIGH | MEDIUM | P1 |
| BYOK encryption | HIGH | MEDIUM | P1 |
| Streaming conversational chat | HIGH | HIGH | P1 |
| Shadow DOM widget + branding | HIGH | MEDIUM | P1 |
| Lead capture in widget | HIGH | LOW | P1 |
| Magic link feedback collection | HIGH | LOW | P1 |
| LLM narrative generation | HIGH | MEDIUM | P1 |
| Per-tenant rate limiting | MEDIUM | MEDIUM | P1 |
| Calibration metrics calculation | MEDIUM | MEDIUM | P1 |
| Git-based prompt versioning | MEDIUM | LOW | P1 |
| Variance display on calibration dashboard | MEDIUM | LOW | P1 |
| Contractor feedback form | MEDIUM | LOW | P1 |
| Audit logging | LOW | LOW | P2 |
| Calibration trend charts | MEDIUM | MEDIUM | P2 |
| Prompt refinement from feedback | HIGH | HIGH | P2 |
| Synthetic data tuning from real outcomes | HIGH | HIGH | P2 |
| Communication coaching UI | HIGH | HIGH | P3 |
| Multiple LLM providers | MEDIUM | HIGH | P3 |
| SSO enterprise login | LOW | HIGH | P3 |
| RLHF fine-tuning | HIGH | HIGH | P3 |

**Priority key:**
- P1: Must have for launch (Epics 3-6)
- P2: Should have, add when possible (v1.x post-validation)
- P3: Nice to have, future consideration (v2+)

---

## Competitor Feature Analysis

| Feature | Buildxact/Procore | Houzz Pro | efOfX Approach |
|---------|--------------|--------------|--------------|
| Estimate generation | Form-based, manual line items | Template-based quotes | Conversational AI chat with RCF methodology |
| Output format | Single number with line items | Single number, e-sign | P50/P80 probabilistic ranges with narrative |
| Embeddability | No embed; standalone web app | No embed; standalone app | White-label widget, <5 lines of code |
| Multi-tenant model | Single-tenant per customer | Single-tenant | True multi-tenant with hard isolation |
| LLM integration | Buildxact Blu for natural language input | None | Full conversational scoping + narrative generation |
| Self-improvement | None; static templates | None | Feedback loop from project outcomes |
| BYOK | N/A (no LLM) | N/A | Tenants bring OpenAI key; platform controls costs |
| Calibration tracking | None | None | Variance metrics, accuracy trending per tenant |

**Key insight:** No competitor combines conversational AI scoping, probabilistic outputs, white-label embeddability, and a self-improving feedback loop. efOfX occupies an uncrowded position — but each component alone (AI estimate tools, contractor widgets, quote software) has multiple competitors. The differentiation is the combination.

---

## Sources

- WorkOS Developer Guide to Multi-Tenant Architecture: https://workos.com/blog/developers-guide-saas-multi-tenant-architecture (MEDIUM confidence — authoritative developer guide)
- Frontegg Multi-tenancy Security Best Practices: https://frontegg.com/blog/saas-multitenancy (MEDIUM confidence — vendor guide, verified against AWS SaaS Lens)
- AWS SaaS Lens — Identity and Access Management: https://docs.aws.amazon.com/wellarchitected/latest/saas-lens/identity-and-access-management.html (HIGH confidence — official AWS prescriptive guidance)
- Dev.to Complete Guide to Streaming LLM Responses: https://dev.to/pockit_tools/the-complete-guide-to-streaming-llm-responses-in-web-applications-from-sse-to-real-time-ui-3534 (MEDIUM confidence — community guide, matches Microsoft Azure Dev Community findings)
- Microsoft Community Hub — Importance of LLM Streaming: https://techcommunity.microsoft.com/blog/azuredevcommunityblog/the-importance-of-streaming-for-llm-powered-chat-applications/4459574 (MEDIUM confidence — Microsoft official blog)
- DEV Community — White Label Chat Complete Guide 2026: https://dev.to/alakkadshaw/white-label-chat-the-complete-guide-to-branded-chat-for-your-website-2026-57e7 (MEDIUM confidence — community guide)
- Datagrid — Building Self-Improving AI Agents with Feedback Loops: https://datagrid.com/blog/7-tips-build-self-improving-ai-agents-feedback-loops (MEDIUM confidence — industry blog, aligns with AI Builder Microsoft Learn docs)
- Baytech Consulting — Magic Links UX, Security and Growth: https://www.baytechconsulting.com/blog/magic-links-ux-security-and-growth-impacts-for-saas-platforms-2025 (MEDIUM confidence — consulting analysis, one source only)
- Braintrust — Best Prompt Versioning Tools 2025: https://www.braintrust.dev/articles/best-prompt-versioning-tools-2025 (MEDIUM confidence — vendor-adjacent but detailed analysis)
- Getmaxim — Prompt Versioning Best Practices 2025: https://www.getmaxim.ai/articles/prompt-versioning-and-its-best-practices-2025/ (MEDIUM confidence — industry source)
- Cybersguards — Shadow DOM Security Guide 2025: https://cybersguards.com/shadow-dom/ (LOW confidence — single source; Shadow DOM browser support confirmed from MDN)
- Buildxact Construction Estimating Software Features: https://www.buildxact.com/us/features/construction-estimating-software/ (MEDIUM confidence — official product page, current)
- Capterra — Best Construction Estimating Software 2026: https://www.capterra.com/construction-estimating-software/ (MEDIUM confidence — aggregator, good for competitive landscape)
- efOfX PRD: docs/PRD.md (HIGH confidence — authoritative source of record for this project)
- efOfX PROJECT.md: .planning/PROJECT.md (HIGH confidence — confirmed validated requirements)
- efOfX Architecture Doc: docs/architecture.md (HIGH confidence — confirmed architectural decisions)

---

*Feature research for: Multi-tenant SaaS estimation platform (LLM + white-label widget + feedback loop)*
*Researched: 2026-02-26*
