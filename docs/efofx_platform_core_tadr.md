# Efofx Platform Core Migration

**Technical Architecture & Decision Record (TADR)**

- **Author:** Brett Lee
- **Date:** 2026-04-22
- **Status:** Approved (Initial Version)
- **Scope:** Migration of Efofx backend logic from FastAPI into a Rust-based platform core aligned with the standard backend architecture for O'Side Systems products

---

## 1. Executive Summary

Efofx is evolving from a product-specific FastAPI backend into a platform-aligned service architecture shared across O'Side Systems backends.

The current Efofx backend already contains meaningful business value, including multi-tenant isolation, structured intake workflows, LLM orchestration, reference-class estimation, lead capture, and feedback collection. That foundation is strong, but the long-term architecture should converge on the same backend model being adopted for CTOsphere: a Rust-based platform core with thin client surfaces and explicit API contracts.

This migration is not driven primarily by performance. It is driven by architectural consistency, platform reuse, cleaner boundaries, stronger contracts, and long-term maintainability.

The target outcome is a single authoritative Rust service that owns domain logic, workflow orchestration, tenant-safe policies, LLM interactions, and estimation behavior; all clients and surfaces will consume that service through documented APIs.

---

## 2. Strategic Intent

Efofx is not merely a chatbot backend.

It is a structured intake and estimation platform that turns vague project requests into estimate-ready leads through dynamic follow-up questions, project normalization, reference-class logic, and explainable estimate generation.

The strategic direction for Efofx is to standardize its backend around the same platform architecture as CTOsphere while preserving the product-specific differentiators that matter:

- multi-tenant isolation
- configurable estimation behavior
- reference-class forecasting
- explainable outputs
- contractor and marketplace integrations
- future feedback-driven calibration loops

This architecture must support multiple client surfaces:

- embeddable widget
- tenant dashboard
- integration APIs for contractor directories and partners
- future CLI, MCP, batch jobs, and internal admin tooling

---

## 3. Core Engineering Principles

### 3.1 Configurable by Default

Anything that can reasonably vary by environment, tenant, vertical, tier, or deployment should be configurable.

This includes, but is not limited to:

- models
- prompt versions
- chat length limits
- token budgets
- rate limits
- enabled features
- supported verticals
- regional multipliers
- routing behavior
- branding defaults
- fallback provider behavior
- feedback rules
- retention periods
- cache TTLs
- pagination defaults
- threshold values
- scoring weights
- status transitions where product rules allow it

There should be no hidden magic strings or unexplained constants in business logic.

### 3.2 YAGNI

Do not introduce generic abstractions or platform subsystems until there is a real second use case.

Examples:

- support one primary HTTP API strategy first
- support one LLM provider path first unless a second is required
- keep background job infrastructure minimal until real async workloads demand more
- do not generalize contractor routing beyond the existing use cases prematurely

### 3.3 DRY

Shared rules must be implemented once, in one authoritative layer.

Examples:

- tenant policy enforcement
- session state transitions
- estimate output envelopes
- auth and authorization checks
- rate-limit policy lookup
- configuration resolution
- feature-flag evaluation

### 3.4 SOLID

The Rust platform core should be built around cohesive modules with explicit interfaces and limited reasons to change.

In practice, this means:

- domain logic separated from transport
- workflow services separated from infrastructure adapters
- storage and provider contracts expressed through traits or equivalent interfaces where justified
- client-specific shaping kept out of core business modules

### 3.5 API-First Contract Discipline

All externally consumed APIs must be documented in OpenAPI.

The OpenAPI specification is not optional documentation added later. It is part of the product contract and the development workflow.

---

## 4. Current State Assessment

The current FastAPI backend is already substantially built. It includes routers for chat, auth, widget flows, feedback, and calibration; service classes for chat, LLM interactions, estimation, RCF matching, prompt loading, auth, BYOK, widget operations, and feedback; and a multi-tenant collection model that structurally prevents cross-tenant leakage.

Important strengths worth preserving:

- hard multi-tenant isolation
- BYOK key management with encryption
- structured estimation outputs
- streaming estimate plus narrative flow
- versioned prompt registry
- embeddable widget with branding and Shadow DOM isolation
- contractor-facing lead capture flow
- seeded reference-class data for outdoor and construction-oriented verticals

Important gaps and active concerns:

- dashboard is incomplete
- contractor routing is not yet implemented
- marketing and deployment are unfinished
- chat-specific abuse limits are not implemented
- monitoring is incomplete
- some integration paths remain stubbed

The pivot analysis also confirms that the real differentiator is not generic chat, but the estimation engine, reference-class logic, location-aware adjustments, explainability, and future feedback loop.

---

## 5. Target Architecture

### 5.1 High-Level Structure

**Client Surfaces:**

- Widget
- Dashboard
- Partner / Integration APIs
- Future CLI / MCP / admin tools

**Rust Platform Core:**

- Domain model
- Intake workflows
- Estimation workflows
- Reference-class engine
- LLM orchestration
- Tenant policy enforcement
- Feedback and calibration logic
- Configuration resolution
- Public API layer

**Infrastructure Layer:**

- MongoDB or successor persistence layer
- LLM provider adapters
- cache layer
- email / notification adapter
- storage / file adapter
- future queue or job execution layer

---

### 5.2 Responsibility Breakdown

**Rust Platform Core owns:**

- tenant-safe domain logic
- chat and intake session lifecycle
- readiness evaluation
- structured scoping extraction
- estimate generation orchestration
- reference-class selection and adjustment logic
- confidence, assumptions, and attribution generation
- lead and consultation workflow rules
- feedback ingestion and calibration rules
- configuration lookup and feature toggles
- public API contracts

**Rust Platform Core does NOT own:**

- widget rendering
- dashboard rendering
- client-local interaction state
- purely presentational formatting

**Web and Client Surfaces** (widget and dashboard) own:

- rendering
- form state
- user interaction flows
- local UX fallback behavior

They do not own domain or workflow logic.

**Integration Surfaces:** Partner integrations consume documented APIs and should not bypass the platform core.

---

## 6. Framework and Runtime Decisions

### 6.1 Primary Rust Framework: Axum

**Decision:** Use Axum as the primary HTTP framework for the Efofx platform core.

**Rationale:**

- consistent with the CTOsphere platform direction
- good fit for explicit API boundaries
- strong middleware ecosystem
- low-magic design that supports maintainability

### 6.2 Async Runtime: Tokio

**Decision:** Use Tokio as the async runtime.

**Rationale:**

- ecosystem standard
- appropriate for IO-heavy API and streaming workloads
- aligns with Axum and future concurrency needs

### 6.3 API Strategy: HTTP/JSON First

**Decision:** Start with HTTP/JSON APIs and defer gRPC unless a clear need emerges.

**Rationale:**

- simpler integration for widget, dashboard, and partner platforms
- easier debugging and API iteration
- OpenAPI-first workflow aligns naturally with HTTP/JSON

### 6.4 Auth Strategy: Supabase for Humans, API Keys for Machines

**Decision:** Outsource all human-user auth to Supabase. Preserve our own per-tenant API key mechanism for machine-to-machine widget authentication.

**Two distinct auth surfaces:**

| Surface | Principal | Mechanism | Who Owns It |
|---------|-----------|-----------|-------------|
| Dashboard, admin UI | Human user | Supabase session → JWT (JWKS-verified in Rust) | Supabase |
| Embeddable widget | Contractor's site | Long-lived tenant API key (hashed, stored on tenant record) | Platform core |

**Rationale:**

- Registration, email verification, password reset, OAuth, MFA, session management are solved by Supabase — no value in re-implementing.
- Widget embedding has no human session; it needs a long-lived bearer credential Supabase cannot provide. We own this.
- Matches YAGNI (§3.2): delete what Supabase gives us; keep what has no equivalent.
- JWKS-based verification keeps secret-handling in Rust to a cached public key, rotatable by Supabase without code changes.

**Scope boundary:**

- Supabase is used **auth-only**. No application data in Postgres. MongoDB remains the system of record (see §8.2).
- On first-seen Supabase user, Rust provisions a `Tenant` record in MongoDB linked by `supabase_user_id`.
- Tenant↔user mapping is **1:1** for v1. Team model is a future migration, not in scope.
- BYOK encryption (§9.3) stays in application code; not delegated to Supabase Vault.

**Project:** The efofx Supabase project is `qzdgqepcdaoyigrgtlzv.supabase.co` (Americas region).

---

## 7. Domain Architecture Decisions

### 7.1 Core Bounded Areas

The platform core should be organized around the following major bounded areas:

- Identity and Tenant Provisioning (Supabase JWT verification + tenant mapping — see §6.4)
- Widget API-Key Authentication (machine-to-machine, owned by the platform core)
- Tenant Configuration
- Widget and Public Intake
- Chat and Session State
- Scoping Extraction
- Estimation and Reference Classes
- Leads and Consultation Requests
- Feedback and Calibration
- Partner Routing and Integrations
- Platform Administration
- Shared Configuration and Policy

### 7.2 Single Source of Truth for Workflow Rules

The Rust service is the authoritative owner of:

- intake readiness rules
- estimate eligibility rules
- lead capture gating behavior
- consultation request rules
- feedback eligibility and acceptance rules
- routing eligibility and derived tags

No client should independently reproduce these rules.

### 7.3 Configuration Over Constants

Any logic that currently depends on magic strings, hard-coded statuses, fixed thresholds, or implicit behavior should move into one of these categories:

- compile-time type-safe enum where the concept is intrinsic and stable
- runtime configuration where tenant, environment, or product behavior may vary
- documented defaults where configuration is optional

Examples:

- rate-limit tiers
- max messages per session
- max tokens per session
- model names
- prompt registry selection
- public branding defaults
- feature toggles for contractor routing
- alpha fallback key behavior
- allowed origins and CORS rules
- analytics event categories where variability is expected

---

## 8. Data and Persistence Decisions

### 8.1 Multi-Tenant Isolation is Non-Negotiable

The current structural tenant isolation is one of the strongest parts of the existing backend and must be preserved in the Rust architecture.

**Decision:**

- every tenant-scoped repository path must require tenant context explicitly
- cross-tenant access must be impossible by default, not merely discouraged by convention
- admin and internal override paths must be isolated and explicit

### 8.2 Database Strategy

**Decision:**

- preserve the document-oriented data model in the near term unless a concrete reason emerges to change persistence technology
- avoid introducing a storage rewrite as part of the Rust migration unless it materially improves the architecture

**Rationale:**

- existing models are already aligned to document-oriented workflows
- the product is still iterating on schema and intake behavior
- storage replacement is not required to achieve the platform architecture

### 8.3 Repository Layer

**Decision:**

- create repository modules per bounded area rather than a generic catch-all data access layer
- keep repository interfaces narrow and use-case driven

---

## 9. LLM and Prompting Decisions

### 9.1 Orchestration Lives in the Platform Core

**Decision:**

- all prompt selection, request shaping, output parsing, retries, and provider interaction live in the Rust platform core

Clients must never directly construct estimation prompts or own business-level parsing rules.

### 9.2 Prompt Versioning Stays Explicit

The existing versioned prompt approach is correct and should remain. New prompt behavior should be introduced through explicit versioning rather than hidden edits.

### 9.3 Provider Strategy

**Decision:**

- keep the provider interface narrow and practical
- preserve BYOK as a first-class capability
- allow platform fallback only where product policy explicitly enables it, such as alpha tiers

### 9.4 Model and Budget Configuration

All of the following must be configurable:

- provider model names
- fallback order
- per-tenant or per-tier token budgets
- cache TTL
- request timeout thresholds
- streaming enablement

---

## 10. API and Contract Decisions

### 10.1 OpenAPI is Mandatory

Every public or partner-consumed HTTP API must be represented in an OpenAPI specification.

This includes at minimum:

- auth endpoints
- chat and estimation endpoints
- lead and consultation endpoints
- feedback endpoints
- widget branding endpoints
- tenant settings endpoints
- contractor routing or partner integration endpoints

### 10.2 Versioned API Surface

**Decision:**

- all externally consumed routes should live under an explicit version namespace
- breaking changes require a new version path or a versioned contract strategy

### 10.3 Consistent Response Envelope

**Decision:**

- adopt a consistent response envelope for synchronous APIs
- use explicit event types for streaming APIs

**Recommended synchronous shape:**

- `data`
- `metadata`
- `errors`

**Recommended streaming shape:**

- `event` type
- `payload`
- `metadata` where appropriate

### 10.4 Partner Integrations are First-Class

The contractor directory integration path is a strategic use case and should be treated as a first-class platform concern, not an ad hoc add-on.

---

## 11. Operational Decisions

### 11.1 Observability

**Decision:**

- add structured logging, tracing, and request correlation from the start of the Rust service
- ensure streaming workflows are observable end to end

The current lack of strong monitoring is a known weakness and should not be carried forward.

### 11.2 Rate Limiting and Abuse Controls

**Decision:**

- preserve existing endpoint rate limiting
- add configurable chat-length and token-budget safeguards as first-class policy controls

This directly aligns with the implementation plan and meeting feedback.

### 11.3 Background Jobs

**Decision:**

- do not introduce a heavy job system until clear needs justify it
- design feedback and calibration workflows so they can later move to async jobs cleanly

This follows YAGNI while preserving a path for future calibration pipelines.

---

## 12. Migration Strategy

### Phase 1: Define Contracts and Boundaries

- identify all current FastAPI business workflows
- define Rust service module boundaries
- define OpenAPI contracts for retained and new endpoints
- classify all constants into typed enums, config, or documented defaults

### Phase 2: Build the Rust Core

- implement tenant-aware auth and config resolution
- implement chat, estimation, lead, feedback, and routing modules
- implement provider and persistence adapters
- implement OpenAPI generation and validation

### Phase 3: Move Client Surfaces to the New APIs

- repoint widget to Rust APIs
- repoint dashboard to Rust APIs
- validate partner and contractor integration flows
- remove duplicated logic from client code

### Phase 4: Hardening

- tracing and logs
- improved error envelopes
- abuse controls
- operational dashboards
- deployment and rollback standards

### Phase 5: Decommission Legacy Backend Paths

- remove or archive superseded FastAPI backend code
- preserve historical docs where useful
- update living docs to the Rust architecture as source of truth

---

## 13. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Over-Abstracting Too Early | Apply YAGNI aggressively; only create shared abstractions with an active second use case |
| Configuration Sprawl | Centralize config schema, defaults, validation, and override precedence |
| Client Drift | Keep all business rules in Rust; client surfaces remain thin |
| API Contract Instability | OpenAPI-first design, versioned APIs, contract tests |
| Platform Consistency Becomes Lowest Common Denominator | Standardize backend architecture without flattening product-specific domain logic |

---

## 14. Definition of Success

The migration is successful when:

- all core Efofx business logic lives in the Rust platform service
- tenant isolation remains structurally enforced
- widget and dashboard are thin clients over documented APIs
- all externally consumed APIs are documented in OpenAPI
- configurable behavior replaces hidden constants and magic strings where appropriate
- partner integrations consume stable contracts
- the architecture remains simpler, not more elaborate, than the product requires

---

## 15. Final Position

Efofx should move to the same platform-standard Rust backend architecture as CTOsphere.

This is not because the existing FastAPI backend is weak. It is because the existing backend is strong enough to justify formalizing it into a more durable platform shape.

The migration should preserve what already works well, especially multi-tenancy, explainable estimation, reference-class logic, BYOK, and structured workflows, while enforcing the engineering standards that will matter across all future O'Side Systems backends:

- **configurable** where variability is real
- **simple** where generality is unnecessary
- **DRY** in shared rules
- **SOLID** in module boundaries
- **OpenAPI** for every externally consumed contract
