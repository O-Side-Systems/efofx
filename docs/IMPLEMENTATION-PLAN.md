# Implementation Plan: Efofx Pivot MVP

**Last Updated:** 2026-04-15
**Goal:** Get efofx to a demo-ready, deployable state for integration with Jeff's contractor-finding website and standalone marketing.
**Approach:** Phased, building on existing codebase. No rewrites — fix what's broken, build what's missing, polish for credibility.

---

## Phase Overview

| Phase | Name | Goal | Depends On |
|-------|------|------|------------|
| 1 | Stabilize & Demo-Ready | Widget works end-to-end without crashing | — |
| 2 | Lead Dashboard MVP | Contractors can see leads and manage settings | Phase 1 |
| 3 | Contractor Routing | Post-estimate flow routes users to contractor directory | Phase 1 |
| 4 | Marketing & Deployment | Live demo site, hosted backend, embeddable widget | Phases 1-2 |
| 5 | Alpha Prep & Polish | Abuse mitigation, platform key fallback, documentation for Jeff | Phases 1-3 |

---

## Phase 1: Stabilize & Demo-Ready

**Objective:** The widget → backend → estimation → narrative flow works reliably end-to-end. No crashes during demo.

### Tasks

#### 1.1 Diagnose and fix the demo crash
- Start the backend locally (`uvicorn app.main:app --reload`)
- Start the widget dev server (`npm run dev`)
- Reproduce the failure from the demo
- Fix the root cause (likely: SSE connection error, CORS issue, or missing env config)
- Test the complete flow: open widget → chat → scoping → lead capture → generate estimate → view narrative

#### 1.2 Verify environment setup
- Ensure `.env` is correctly configured (MongoDB Atlas URI, OpenAI key, encryption keys, JWT secret)
- Verify MongoDB connection and indexes
- Verify prompt files load correctly at startup
- Test with a real OpenAI API key (not just mocked)

#### 1.3 Fix widget reliability
- Test floating mode and inline mode
- Verify Shadow DOM isolation doesn't break on real host pages
- Ensure branding fetch works (public endpoint, no auth)
- Test SSE streaming stability (narrative tokens, error events, reconnection)
- Test error states: invalid API key (402), rate limiting (429), network failures

#### 1.4 End-to-end smoke test
- Full flow: register tenant → login → set BYOK key → embed widget → chat → estimate → lead captured
- Verify lead appears in MongoDB
- Verify analytics events recorded

### Definition of Done
- Widget loads and runs without errors in both floating and inline modes
- Chat → estimate → narrative flow completes successfully
- Lead capture form submits and data persists
- No console errors, no broken SSE connections

---

## Phase 2: Lead Dashboard MVP

**Objective:** Contractors can log in and see their leads, manage their branding, and configure their BYOK key through a UI.

### Tasks

#### 2.1 Fix dashboard auth flow
- Wire Login page to `/auth/login` endpoint
- Implement JWT token storage (localStorage or secure cookie)
- Add auth header injection to API client (`apps/efofx-dashboard/src/api/client.ts`)
- Add protected route wrapper
- Test login → dashboard → logout flow

#### 2.2 Build lead list page
- New page: `/leads`
- Fetch from `/api/v1/widget/lead` or add a new `GET /api/v1/leads` endpoint if needed
- Display: date, contact name, email, phone, project type, estimate range, status
- Click to view lead detail (conversation history, full estimate)
- Basic filtering: by date range, by project type

#### 2.3 Build lead detail view
- Show: contact info, project summary (from scoping context), full estimate (EstimationOutput), conversation transcript
- Link to consultation request if one exists
- Status management: new → contacted → converted → closed

#### 2.4 Build tenant settings page
- New page: `/settings`
- **Branding section:** primary/secondary/accent colors, logo URL, welcome message, button text
  - Preview component showing how widget will look
  - Save to tenant settings via `PATCH /auth/profile`
- **BYOK section:** current key status (masked), update key form
  - Uses existing `POST /auth/openai-key` endpoint
- **Widget embed section:** show embed code snippet with tenant's API key
  - Copy-to-clipboard button

#### 2.5 Dashboard navigation
- Add sidebar or top nav: Dashboard (calibration), Leads, Settings
- Keep existing calibration dashboard as a tab/page

### Backend Changes Needed
- Add `GET /api/v1/leads` endpoint (list leads for tenant with pagination/filtering)
- Add `GET /api/v1/leads/{lead_id}` endpoint (single lead with conversation history)
- Add `PATCH /api/v1/leads/{lead_id}` endpoint (update lead status)
- Potentially: add lead status field to widget_leads collection if not present

### Definition of Done
- Contractor can log in, see their leads, view lead details
- Contractor can update branding colors and see embed code
- Contractor can add/update their BYOK OpenAI key
- Data persists across sessions

---

## Phase 3: Contractor Routing

**Objective:** After estimate generation, the widget can route users to relevant contractors on Jeff's directory site.

### Tasks

#### 3.1 Define contractor routing data model
- Project type tags/categories (already partially exist in ScopingContext.project_type)
- Mapping from project types to contractor specialties
- Routing URL template (e.g., `https://jeffs-site.com/contractors?specialty={tag}&location={location}`)

#### 3.2 Add routing configuration to tenant settings
- New field in tenant settings: `routing_config`
  - `enabled`: boolean
  - `base_url`: URL template for contractor directory
  - `tag_mapping`: optional override of project_type → tag mapping
- API endpoint to update routing config (via existing `PATCH /auth/profile`)

#### 3.3 Include routing data in estimate response
- Add `routing_tags` to EstimationOutput or as a separate field in the SSE `done` event
- Tags derived from: scoping_context.project_type, location, estimated cost tier

#### 3.4 Build "Find Contractors" CTA in widget
- New component: `ContractorRoutingCTA`
- Appears after estimate is displayed (result phase)
- Button: "Find contractors for this project" → opens routing URL in new tab
- Customizable text via branding config

#### 3.5 Integration API for directory sites
- New endpoint: `POST /api/v1/integration/contractor-match`
  - Input: project type, location, estimated budget range
  - Output: search parameters formatted for the directory
- This allows Jeff's site to receive structured data from efofx, not just a redirect

### Definition of Done
- After estimate, user sees "Find Contractors" button
- Clicking navigates to contractor directory with appropriate filters
- Routing is configurable per tenant

---

## Phase 4: Marketing & Deployment

**Objective:** efofx is accessible online with a marketing page and live demo.

### Tasks

#### 4.1 Deploy backend
- Set up DigitalOcean droplet (or App Platform) for efofx-estimate
- Configure production environment variables
- Set up MongoDB Atlas production cluster (or use existing)
- Configure domain (e.g., api.efofx.ai)
- Verify health endpoint accessible

#### 4.2 Build and deploy widget bundle
- Run `npm run build` for efofx-widget
- Host `embed.js` on CDN (DigitalOcean Spaces or similar)
- Verify embed works on external page

#### 4.3 Build marketing/demo site
- Simple single-page site:
  - Hero: "Turn vague project requests into estimate-ready leads in minutes"
  - How it works: 3-step visual (describe → chat → estimate)
  - Live demo: embedded widget in inline mode with a demo tenant
  - Features list
  - Pricing: "Alpha — free for early partners" or "Contact us"
  - CTA: "Try the demo" / "Get in touch"
- Tech: simple static site (HTML/Tailwind, or minimal React)
- Host on DigitalOcean Spaces / Netlify / Vercel

#### 4.4 Set up demo tenant
- Create a demo tenant in production DB
- Configure branding for the marketing site
- Provide a capped OpenAI key for the demo
- Set appropriate rate limits

### Definition of Done
- Marketing site live with working demo widget
- Backend API accessible and healthy
- Widget embeddable from CDN

---

## Phase 5: Alpha Prep & Polish

**Objective:** Ready for Jeff to integrate and for alpha testing with real users.

### Tasks

#### 5.1 Chat length limits
- Add configurable `max_messages_per_session` to tenant settings (default: 20)
- Enforce in ChatService — return friendly message when limit reached
- Add `max_tokens_per_session` as secondary safeguard
- Jeff's suggestion: simple, prevents abuse without complex systems

#### 5.2 Platform key fallback for alpha
- Add `platform_openai_key` to config (separate from per-tenant BYOK)
- In `get_llm_service()`: if tenant has no BYOK key AND tenant.tier == "alpha", use platform key
- Add monthly token budget tracking per tenant when using platform key
- Alert when approaching budget cap

#### 5.3 Error handling improvements
- Improve error messages shown to end users in widget
- Add reconnection logic for dropped SSE connections
- Better handling of OpenAI quota exhaustion (clear user-facing message)
- Add widget-level error reporting (optional Sentry or log endpoint)

#### 5.4 Documentation for Jeff
- Integration guide: how to embed the widget on his contractor-finding site
- API reference for the contractor routing integration
- Branding customization guide
- Troubleshooting common issues

#### 5.5 Jeff-specific customizations
- Review Jeff's contractor directory requirements
- Customize tag mapping for his contractor categories
- Test end-to-end: Jeff's site → efofx widget → estimate → route to contractors
- Verify mobile responsiveness of widget on his site

### Definition of Done
- Chat abuse is mitigated with length limits
- Alpha tenants can use platform key with caps
- Jeff has documentation and working integration
- Widget handles errors gracefully

---

## Technical Debt to Track (Not Blocking MVP)

These should be addressed post-alpha but documented for awareness:

| Item | Description | Risk if Deferred |
|------|-------------|-----------------|
| No frontend tests | Widget and dashboard have zero automated tests | Regressions during rapid iteration |
| Image upload unused | Endpoint exists but no vision model integration | Feature confusion |
| MCP functions incomplete | Architecture defined, endpoints stubbed | Not blocking — backend handles estimation directly |
| Calibration pipeline | Data model exists, aggregation incomplete | Can't measure accuracy improvement |
| Monitoring | Sentry removed, no replacement | Blind to production errors |
| estimator-project app | Legacy duplicate of efofx-estimate | Repo clutter |
| Multi-LLM support | Only OpenAI currently | Vendor lock-in (acceptable for alpha) |

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-04-15 | Keep BYOK as primary, add platform key fallback for alpha | BYOK is fully built; alpha needs frictionless onboarding |
| 2026-04-15 | Don't rewrite — fix and extend existing codebase | ~60% of PRD features already implemented |
| 2026-04-15 | Prioritize widget stability over new features | Demo crashed — credibility requires reliability first |
| 2026-04-15 | Build lead dashboard before contractor routing | Contractors need to see value (leads) before integration complexity |
| 2026-04-15 | Single vertical (outdoor/construction) for alpha | Matches reference class data, Jeff's market, lower complexity |
| 2026-04-15 | Archive old docs, maintain living docs in docs/ root | Old docs from Nov 2025 significantly outdated |
