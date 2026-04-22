# efOfX Estimation Service - Product Requirements Document

**Author:** Brett
**Date:** 2025-11-09
**Version:** 1.0

---

## Executive Summary

The efOfX Estimation Service is an AI-powered, multi-tenant SaaS platform that transforms project estimation from guesswork into transparent, data-driven forecasting. Using Reference Class Forecasting (RCF) methodology combined with LLM-powered narrative generation, efOfX helps businesses provide accurate project estimates while teaching them how to communicate uncertainty effectively to stakeholders.

**MVP Focus:** Construction and home improvement estimation with domain-agnostic backend architecture, synthetic reference data generation, and feedback-driven model refinement.

**Vision:** A "communication coach that happens to estimate projects" - building trust through transparency rather than false precision.

### What Makes This Special

efOfX isn't just another estimation tool. It's built on three breakthrough principles from extensive brainstorming:

1. **Trust Through Transparency**: The "two-layer model" separates internal truth (P50/P80 ranges, assumptions, risks) from external communication (stakeholder-specific narratives). "The performance layer isn't a mask—it's a mirror."

2. **Self-Improving System**: Closes the feedback loop between estimates and reality. Every completed project makes the next estimate better through customer and contractor feedback forms.

3. **Domain-Agnostic by Design**: Generic backend architecture supports any estimation domain (construction → IT/dev → finance → healthcare). Same engine, different reference data.

The competitive moat is **trust, not features**. Competitors can clone features, but trust is earned through transparent communication and calibrated accuracy over time.

---

## Project Classification

**Technical Type:** SaaS B2B Backend/API
**Primary Domain:** Construction & Home Improvement (MVP), IT/Development (Fast Follow)
**Complexity:** Medium (Generic architecture with domain-specific reference data)

**Architecture Pattern:**
- Multi-tenant FastAPI backend (Python)
- Serverless MCP functions for reference class queries (Node.js)
- Shared MongoDB Atlas database
- OpenAI LLM integration with BYOK (Bring Your Own Key)

**Project State:** Brownfield - Core architecture exists, adding MVP features:
- Synthetic data generation
- Feedback/calibration system
- Enhanced RCF engine
- Domain-generic backend design

---

## Success Criteria

### What MVP Success Looks Like

Success for efOfX MVP is measured by **trust, accuracy, and distribution** - not vanity metrics.

#### 1. Calibration Accuracy (The Core Promise)
- **70% of estimates within 20% of actual outcomes** for construction domain
- Measurable improvement in accuracy after feedback incorporation
- Track record visible to users: "Within 15% for 68% of similar pool installations"

#### 2. Feedback Loop Closure (Self-Improvement)
- **40% of completed projects submit actual outcome data**
- Both customer AND contractor feedback captured for discrepancies
- System learns from every submission, visible in improved estimates
- Calibration metrics displayed per tenant: "Your estimates are trending 12% high"

#### 3. Distribution Success (White Label Adoption)
- **10 contractors embed the white label chat widget** on their websites
- Widget drives at least 50 estimation requests across all installations
- Contractors report: "Customers prefer the interactive chat over static quote forms"
- Zero security incidents or cross-tenant data leakage

#### 4. Multi-Tenant Operations (Business Viability)
- **15 active tenants using BYOK** for construction estimates
- Each tenant has submitted at least 5 estimates
- At least 5 tenants have closed the feedback loop (submitted outcomes)
- Synthetic data proven viable (real outcomes validate synthetic reference classes)

#### 5. Trust Building (The Differentiator)
- Users choose **transparency mode** (P50/P80 ranges) over "single number" estimates
- Contractors use estimates to **educate customers**, not just quote
- Reduced estimate padding over time (measurable through feedback data)
- Customer feedback: "I understood why the range existed"

#### 6. Technical Foundation (Enables Scale)
- Domain-agnostic backend proven with **2 verticals** (construction + IT/dev)
- Synthetic data generation creates viable reference classes (validated by real outcomes)
- BYOK multi-tenancy works securely with encrypted API keys
- White label widget works on contractor sites with <5 lines of JavaScript

### What MVP Does NOT Need to Achieve

- ❌ Communication coaching UI (internal/external narrative toggle - post-MVP)
- ❌ Multiple LLM provider support (OpenAI BYOK is sufficient for MVP)
- ❌ Real-time collaboration (async estimation is fine)
- ❌ Mobile apps (web-based widget + API is sufficient)
- ❌ Advanced analytics dashboard (basic calibration metrics only)

### Success Tracking

**Key Metrics Dashboard:**
- Estimation requests per tenant
- Feedback submission rate
- Calibration accuracy by reference class
- White label widget adoption rate
- Synthetic vs real data performance comparison

---

## Product Scope

### MVP - Minimum Viable Product

**Goal:** Prove the core value proposition with construction domain, establish technical foundation for multi-domain expansion.

#### Core Estimation Engine (Foundational RCF)
- **Reference Class Forecasting (RCF) Engine**
  - Match user project description to best reference class
  - Calculate baseline estimates from reference projects
  - Apply regional and complexity adjustments
  - Return P50/P80 cost and timeline distributions

- **Synthetic Data Generation System**
  - Generate realistic reference classes for construction/home improvement
  - Project types: Pools, ADUs, Kitchen Remodel, Bathroom Remodel, Landscaping, Roofing, Flooring
  - Regional variations: SoCal Coastal, SoCal Inland, NorCal, Central Coast
  - Cost distributions (P50/P80/P95), timeline distributions, breakdown templates
  - Validation: Synthetic data must be within 25% of real-world norms

- **Domain-Agnostic Backend Architecture**
  - Reference classes stored with flexible schema (not hardcoded to construction)
  - Generic category/subcategory/region model
  - Support for custom attributes per domain
  - Tenant-specific reference class libraries

#### Multi-Tenant Infrastructure
- **Tenant Management**
  - Tenant registration and authentication (JWT-based)
  - Per-tenant OpenAI API key storage (encrypted, BYOK)
  - Tenant isolation at database query level
  - Rate limiting per tenant tier

- **API Security**
  - HMAC + JWT dual authentication (existing)
  - Encrypted API key storage
  - Zero cross-tenant data leakage
  - Audit logging per tenant

#### LLM Integration (OpenAI BYOK)
- **Estimate Narrative Generation**
  - Generate stakeholder-friendly explanations
  - Explain assumptions and risks
  - Suggest questions to ask contractor
  - Adapt tone for customer vs contractor audience

- **Conversational Scoping (Chat)**
  - Iterative project description gathering
  - Extract key details: size, location, scope, timeline
  - Clarifying questions based on missing information
  - Determine when enough info exists for estimate

- **Bring Your Own Key (BYOK)**
  - Tenants provide their own OpenAI API keys
  - Encrypted storage per tenant
  - Fallback to platform key for trials

#### Feedback & Calibration System
- **Customer Feedback Form**
  - Submit actual project cost (final invoice)
  - Submit actual timeline (start/completion dates)
  - Rate accuracy (1-5 stars)
  - Open comments on what changed

- **Contractor Feedback Form**
  - Submit contractor's actual cost breakdown
  - Explain discrepancies from customer report
  - Flag estimate issues (scope creep, hidden costs, market changes)
  - Suggest reference class improvements

- **Calibration Engine**
  - Calculate variance: (actual - estimated) / estimated
  - Track accuracy by reference class
  - Display tenant-level calibration metrics
  - Use feedback to tune future estimates (LLM prompt refinement)

#### White Label Chat Widget (Distribution)
- **Embeddable JavaScript Widget**
  - <5 lines of code to embed on contractor website
  - Branded with contractor's colors/logo
  - Conversational UI for project scoping
  - Generates estimate in real-time

- **Widget Configuration**
  - Tenant can customize branding (logo, colors, button text)
  - Configurable lead capture (email, phone)
  - Optional: Embed estimate in email to customer

- **Security & Privacy**
  - Widget uses tenant's API key (BYOK)
  - No efOfX branding visible to end customer
  - Customer data stored under tenant's account
  - CORS and CSP compliant

#### MCP Server (Reference Class Queries)
- **Serverless Functions** (Existing, enhancements)
  - Query reference classes by attributes
  - Retrieve single reference class by ID
  - Apply regional/complexity adjustments
  - Cached responses (5min TTL)

- **Performance**
  - p95 response time < 150ms (warm)
  - Cold start < 300ms
  - LRU caching for popular queries

#### API Endpoints (FastAPI Backend)
- **Estimation API**
  - POST `/estimate/start` - Initiate estimation session
  - GET `/estimate/{session_id}` - Retrieve estimate
  - POST `/estimate/{session_id}/refine` - Update with more details

- **Chat API**
  - POST `/chat/send` - Send message in scoping conversation
  - GET `/chat/{session_id}/history` - Retrieve conversation

- **Feedback API**
  - POST `/feedback/customer` - Submit customer outcome
  - POST `/feedback/contractor` - Submit contractor outcome
  - GET `/feedback/summary` - Tenant calibration metrics

- **Widget API**
  - GET `/widget/config` - Retrieve tenant branding config
  - POST `/widget/estimate` - Widget-specific estimation endpoint

### Growth Features (Post-MVP)

**Fast Follow: IT/Development Estimation Domain**
- Reference classes for software projects
  - API development, mobile apps, web apps, integrations, migrations
- User (Brett) provides real project data to validate/tune
- Same backend, different reference data
- Proves domain-agnostic architecture

**Enhanced Calibration**
- Automated LLM fine-tuning based on feedback
- Reference class evolution (split/merge based on accuracy)
- Outlier detection and explanation
- Confidence scoring per estimate

**Advanced White Label**
- Multi-language widget support
- SMS/WhatsApp integration
- Voice-based scoping (Twilio integration)
- CRM integrations (HubSpot, Salesforce)

**Contractor Tools**
- Estimate history and analytics
- Proposal generation from estimates
- Customer communication templates
- Win/loss tracking

### Vision (Future)

**Communication Coaching Layer**
- Two-layer model UI: Toggle between "Internal Truth" and "External Narrative"
- Audience-specific framing (CEO, CFO, Client, Dev Team)
- Coaching prompts: "Your CFO will ask about variance - here's how to frame it"
- Truth Integrity Score: "92% - all major risks disclosed"

**Multi-Domain Platform**
- Healthcare/medical procedures
- Legal case estimation
- Financial services projects
- Manufacturing production runs
- Event planning and production

**AI-Powered Insights**
- Pattern detection across estimates
- Risk prediction before project starts
- Market trend analysis (costs rising in specific categories)
- Recommendation engine: "Similar projects succeeded with..."

**Marketplace & Community**
- Tenant-contributed reference classes (anonymized)
- Industry benchmarking
- Best practice sharing
- Certified estimator network

---

## Scope Boundaries

### What's Explicitly OUT of MVP Scope

- ❌ **Mobile native apps** (web widget is sufficient)
- ❌ **Real-time collaboration** (async estimation only)
- ❌ **Multiple LLM providers** (OpenAI only via BYOK)
- ❌ **Advanced analytics dashboard** (basic calibration metrics only)
- ❌ **CRM integrations** (manual export only)
- ❌ **Payment processing** (SaaS billing only, no project payments)
- ❌ **Project management** (estimation only, not tracking)
- ❌ **Contractor marketplace** (tenants find their own customers)

### MVP Constraints

- **Single LLM:** OpenAI only (GPT-4)
- **Two domains:** Construction + IT/Dev
- **Web-only:** No mobile apps
- **English only:** No i18n
- **Basic UI:** Focus on API + widget, minimal admin dashboard

---

## SaaS B2B Backend Specific Requirements

### Multi-Tenancy Architecture

**Tenant Isolation Model:**
- **Hard Isolation:** All database queries scoped by `tenant_id`
- **Data Separation:** Each tenant's data completely isolated (estimates, feedback, widget configs)
- **API Key Isolation:** Encrypted OpenAI keys stored per tenant, never shared
- **Rate Limiting:** Configurable per tenant tier (free/pro/enterprise)

**Tenant Management:**
- Tenant registration with email verification
- API key generation and rotation
- Tenant settings (region preferences, branding, BYOK configuration)
- Tenant deactivation (soft delete, data retention for 90 days)

**Tenant Tiers:**
| Tier | Rate Limit | BYOK Required | White Label | Max Reference Classes |
|------|------------|---------------|-------------|----------------------|
| Trial | 10/hour | No (platform key) | No | Platform only |
| Pro | 100/hour | Yes | Yes | Platform + 50 custom |
| Enterprise | 1000/hour | Yes | Yes | Unlimited custom |

### Authentication & Authorization

**API Authentication (Existing, Enhanced):**
- **User → FastAPI:** JWT bearer tokens with tenant_id claim
- **FastAPI → MCP Functions:** HMAC + short-lived JWT (dual auth)
- **Widget → Backend:** Tenant API key in widget config, session tokens for end users

**BYOK (Bring Your Own Key) Security:**
- OpenAI API keys encrypted at rest (AES-256)
- Keys decrypted only at request time in memory
- Never logged or transmitted in plaintext
- Key validation on tenant setup (test API call)

**Access Control:**
- Admin users can manage tenant settings
- Standard users can create estimates and view calibration data
- Read-only users can view estimates only
- Widget users (end customers) have session-scoped access only

### Data Schemas

**Core Data Models (MongoDB Collections):**

1. **tenants**
   ```json
   {
     "_id": ObjectId,
     "name": "Acme Contractors",
     "email": "admin@acme.com",
     "tier": "pro",
     "api_key_hash": "...",
     "openai_api_key_encrypted": "...",
     "region_preferences": ["SoCal - Coastal"],
     "branding": {
       "logo_url": "https://...",
       "primary_color": "#FF5733",
       "widget_button_text": "Get Estimate"
     },
     "rate_limit_tier": "pro",
     "is_active": true,
     "created_at": ISODate,
     "updated_at": ISODate
   }
   ```

2. **reference_classes** (Domain-Agnostic Schema)
   ```json
   {
     "_id": ObjectId,
     "tenant_id": ObjectId, // null for platform-provided
     "category": "construction",
     "subcategory": "pool",
     "name": "Residential Pool - Midrange",
     "description": "In-ground pool installation...",
     "keywords": ["pool", "swimming", "backyard"],
     "regions": ["SoCal - Coastal", "SoCal - Inland"],
     "attributes": {  // Domain-specific, flexible
       "size_range": "15x30 to 20x40 ft",
       "depth": "4-8 ft",
       "includes_spa": false
     },
     "cost_distribution": {
       "P50": 75000,
       "P80": 92000,
       "P95": 115000,
       "currency": "USD"
     },
     "timeline_distribution": {
       "P50": 8,
       "P80": 10,
       "P95": 14,
       "unit": "weeks"
     },
     "cost_breakdown_template": {
       "materials": 0.40,
       "labor": 0.30,
       "equipment": 0.10,
       "permits": 0.05,
       "design": 0.05,
       "contingency": 0.10
     },
     "regional_multipliers": {
       "SoCal - Coastal": 1.15,
       "SoCal - Inland": 1.05,
       "NorCal": 1.20
     },
     "is_synthetic": true,
     "validation_source": "Industry averages 2024",
     "created_at": ISODate,
     "updated_at": ISODate
   }
   ```

3. **estimations**
   ```json
   {
     "_id": ObjectId,
     "session_id": "uuid-v4",
     "tenant_id": ObjectId,
     "user_description": "I want to install a 15x30 pool...",
     "extracted_attributes": {
       "category": "construction",
       "subcategory": "pool",
       "region": "SoCal - Coastal",
       "size": "15x30 ft",
       "includes_spa": true
     },
     "matched_reference_class_id": ObjectId,
     "confidence_score": 0.87,
     "estimate": {
       "cost_p50": 78000,
       "cost_p80": 95000,
       "timeline_weeks_p50": 8,
       "timeline_weeks_p80": 11,
       "cost_breakdown": {
         "materials": 31200,
         "labor": 23400,
         "equipment": 7800,
         "permits": 3900,
         "design": 3900,
         "contingency": 7800
       }
     },
     "assumptions": [
       "Standard soil conditions",
       "No existing structure removal",
       "Standard permit process"
     ],
     "risks": [
       "Rock/hardpan soil could add $5-15k",
       "Permit delays could add 2-4 weeks"
     ],
     "llm_narrative": "Based on similar projects...",
     "chat_history": [...],
     "widget_session": true,
     "customer_email": "customer@example.com", // if captured
     "status": "completed", // draft, completed, feedback_submitted
     "created_at": ISODate,
     "updated_at": ISODate
   }
   ```

4. **feedback**
   ```json
   {
     "_id": ObjectId,
     "estimation_id": ObjectId,
     "tenant_id": ObjectId,
     "feedback_type": "customer", // or "contractor"
     "actual_cost": 82500,
     "actual_timeline_weeks": 9,
     "variance_cost_pct": 5.8,
     "variance_timeline_pct": 12.5,
     "accuracy_rating": 4, // 1-5
     "comments": "Soil was harder than expected...",
     "discrepancy_flags": {
       "scope_creep": false,
       "hidden_costs": true,
       "market_changes": false
     },
     "submitted_by": "customer", // or contractor email
     "submitted_at": ISODate
   }
   ```

5. **widget_sessions** (End-customer interactions)
   ```json
   {
     "_id": ObjectId,
     "session_token": "uuid-v4",
     "tenant_id": ObjectId,
     "estimation_id": ObjectId, // linked when estimate created
     "messages": [
       {"role": "assistant", "content": "Hi! Let's scope your project..."},
       {"role": "user", "content": "I need a pool"}
     ],
     "customer_info": {
       "email": "customer@example.com",
       "phone": "+1234567890",
       "captured_at": ISODate
     },
     "widget_config_snapshot": {...}, // branding at time of session
     "expires_at": ISODate, // 24 hours from creation
     "created_at": ISODate
   }
   ```

### API Endpoints (Complete Specification)

#### Tenant Management API

**POST /tenants/register**
- **Purpose:** Register new tenant account
- **Auth:** None (public)
- **Request:**
  ```json
  {
    "name": "Acme Contractors",
    "email": "admin@acme.com",
    "password": "...",
    "tier": "trial"
  }
  ```
- **Response:** 201 Created
  ```json
  {
    "tenant_id": "...",
    "api_key": "efofx_...",
    "message": "Account created. Verify email to activate."
  }
  ```

**PUT /tenants/settings**
- **Purpose:** Update tenant configuration
- **Auth:** JWT (admin role)
- **Request:**
  ```json
  {
    "openai_api_key": "sk-...",
    "region_preferences": ["SoCal - Coastal"],
    "branding": {
      "logo_url": "https://...",
      "primary_color": "#FF5733"
    }
  }
  ```
- **Response:** 200 OK

#### Estimation API

**POST /estimate/start**
- **Purpose:** Initiate new estimation session
- **Auth:** JWT or Widget Session Token
- **Request:**
  ```json
  {
    "description": "I want to install a 15x30 pool with spa in my backyard",
    "region": "SoCal - Coastal",
    "customer_email": "customer@example.com" // optional, for widget
  }
  ```
- **Response:** 200 OK
  ```json
  {
    "session_id": "uuid-v4",
    "matched_reference_class": {
      "name": "Residential Pool - Midrange",
      "confidence": 0.87
    },
    "estimate": {
      "cost_p50": 78000,
      "cost_p80": 95000,
      "timeline_weeks_p50": 8,
      "timeline_weeks_p80": 11,
      "cost_breakdown": {...}
    },
    "assumptions": [...],
    "risks": [...],
    "narrative": "Based on 47 similar pool projects..."
  }
  ```

**GET /estimate/{session_id}**
- **Purpose:** Retrieve estimate by session ID
- **Auth:** JWT or Widget Session Token
- **Response:** 200 OK (same structure as POST response)

**POST /estimate/{session_id}/refine**
- **Purpose:** Update estimate with additional details
- **Auth:** JWT or Widget Session Token
- **Request:**
  ```json
  {
    "additional_info": "Pool should be heated, with automatic cover"
  }
  ```
- **Response:** 200 OK (updated estimate)

#### Chat API (Conversational Scoping)

**POST /chat/send**
- **Purpose:** Send message in scoping conversation
- **Auth:** JWT or Widget Session Token
- **Request:**
  ```json
  {
    "session_id": "uuid-v4",
    "message": "I need a pool"
  }
  ```
- **Response:** 200 OK
  ```json
  {
    "session_id": "uuid-v4",
    "response": "Great! What size pool are you thinking? Standard residential pools are usually 15x30 to 20x40 feet.",
    "suggestions": [
      "15x30 feet",
      "20x40 feet",
      "Not sure, need recommendations"
    ],
    "ready_for_estimate": false,
    "extracted_details": {
      "category": "construction",
      "subcategory": "pool"
    }
  }
  ```

**GET /chat/{session_id}/history**
- **Purpose:** Retrieve full conversation history
- **Auth:** JWT or Widget Session Token
- **Response:** 200 OK
  ```json
  {
    "messages": [
      {"role": "assistant", "content": "..."},
      {"role": "user", "content": "..."}
    ],
    "ready_for_estimate": true,
    "extracted_details": {...}
  }
  ```

#### Feedback API

**POST /feedback/customer**
- **Purpose:** Submit customer's actual project outcome
- **Auth:** Public (magic link) or JWT
- **Request:**
  ```json
  {
    "estimation_id": "uuid-v4",
    "actual_cost": 82500,
    "actual_timeline_weeks": 9,
    "accuracy_rating": 4,
    "comments": "Soil was harder than expected, added $4k"
  }
  ```
- **Response:** 201 Created

**POST /feedback/contractor**
- **Purpose:** Submit contractor's actual breakdown
- **Auth:** JWT (tenant admin)
- **Request:**
  ```json
  {
    "estimation_id": "uuid-v4",
    "actual_cost": 82500,
    "actual_timeline_weeks": 9,
    "actual_breakdown": {
      "materials": 35000,
      "labor": 28000,
      "equipment": 8500,
      "permits": 4000,
      "design": 3000,
      "contingency": 4000
    },
    "discrepancy_explanation": "Encountered rock layer, required additional excavation"
  }
  ```
- **Response:** 201 Created

**GET /feedback/summary**
- **Purpose:** Retrieve tenant's calibration metrics
- **Auth:** JWT (tenant)
- **Response:** 200 OK
  ```json
  {
    "total_estimates": 47,
    "feedback_submissions": 21,
    "feedback_rate": 0.45,
    "accuracy_metrics": {
      "avg_cost_variance_pct": 8.2,
      "avg_timeline_variance_pct": 12.5,
      "within_20_pct": 0.71
    },
    "by_reference_class": {
      "Residential Pool - Midrange": {
        "count": 12,
        "avg_variance_pct": 6.5,
        "within_20_pct": 0.83
      }
    }
  }
  ```

#### Widget API

**GET /widget/config**
- **Purpose:** Retrieve tenant's widget branding configuration
- **Auth:** Tenant API key (in query param)
- **Request:** `GET /widget/config?api_key=efofx_...`
- **Response:** 200 OK
  ```json
  {
    "branding": {
      "logo_url": "https://...",
      "primary_color": "#FF5733",
      "button_text": "Get Your Estimate"
    },
    "session_token": "uuid-v4" // for subsequent widget requests
  }
  ```

**POST /widget/estimate**
- **Purpose:** Widget-specific estimation endpoint (simplified)
- **Auth:** Widget Session Token
- **Request:**
  ```json
  {
    "session_token": "uuid-v4",
    "description": "15x30 pool with spa",
    "customer_email": "customer@example.com"
  }
  ```
- **Response:** 200 OK (same as /estimate/start)

#### Synthetic Data API (Admin/Internal)

**POST /admin/synthetic/generate**
- **Purpose:** Generate synthetic reference classes
- **Auth:** JWT (admin role)
- **Request:**
  ```json
  {
    "category": "construction",
    "subcategories": ["pool", "adu", "kitchen_remodel"],
    "regions": ["SoCal - Coastal", "SoCal - Inland"],
    "count_per_subcategory": 3
  }
  ```
- **Response:** 201 Created
  ```json
  {
    "generated_count": 6,
    "reference_class_ids": [...]
  }
  ```

**GET /admin/synthetic/validate**
- **Purpose:** Compare synthetic data to real outcomes
- **Auth:** JWT (admin role)
- **Response:** 200 OK
  ```json
  {
    "synthetic_classes": 15,
    "validated_by_feedback": 8,
    "avg_variance_from_real": 12.3,
    "classes_needing_tuning": [
      {
        "id": "...",
        "name": "ADU Construction",
        "variance": 28.5,
        "recommendation": "Adjust P80 upward by 15%"
      }
    ]
  }
  ```

---

## Functional Requirements

### FR-1: Reference Class Forecasting Engine

**FR-1.1: Reference Class Matching**
- **Requirement:** System shall match user project description to best reference class using keyword matching, category detection, and region filtering
- **Acceptance Criteria:**
  - Confidence score returned with each match (0-1.0)
  - If confidence < 0.7, prompt user for more details
  - Support partial matching (e.g., "pool" matches "Residential Pool - Midrange")
  - Handle multi-word queries ("backyard swimming pool")
- **Domain Constraint:** Must work generically across construction and IT/dev domains

**FR-1.2: Baseline Estimate Calculation**
- **Requirement:** System shall calculate cost and timeline estimates from matched reference class distributions
- **Acceptance Criteria:**
  - Return P50 and P80 values for cost and timeline
  - Apply regional multipliers based on user's region
  - Generate cost breakdown using template percentages
  - Include variance ranges: "P50: $78k, P80: $95k (±$17k range)"
- **Domain Constraint:** Cost breakdown categories must be configurable per domain

**FR-1.3: Adjustment Application**
- **Requirement:** System shall apply complexity and risk adjustments to baseline estimates
- **Acceptance Criteria:**
  - Complexity factor: 0.8x (simple) to 1.5x (complex)
  - Risk factor: 1.0x (low risk) to 1.3x (high risk)
  - Adjustments applied multiplicatively: final = baseline × complexity × risk × regional
  - Show calculation breakdown in response
- **Domain Constraint:** Adjustment factors must be tunable per tenant

**FR-1.4: Assumptions and Risks Extraction**
- **Requirement:** System shall generate assumptions and risks using LLM based on reference class and project details
- **Acceptance Criteria:**
  - Minimum 3 assumptions per estimate
  - Minimum 2 risks per estimate
  - Risks include impact estimate ("could add $5-15k")
  - Assumptions are verifiable ("Standard soil conditions")
- **Domain Constraint:** LLM prompts must be domain-aware

### FR-2: Synthetic Data Generation

**FR-2.1: Construction Reference Class Generation**
- **Requirement:** System shall generate realistic synthetic reference classes for construction domain
- **Acceptance Criteria:**
  - 7 subcategories supported: pool, adu, kitchen_remodel, bathroom_remodel, landscaping, roofing, flooring
  - 4 regions supported: SoCal Coastal, SoCal Inland, NorCal, Central Coast
  - Cost distributions based on 2024 industry averages
  - Timeline distributions realistic for each project type
  - Marked as `is_synthetic: true` in database
- **Validation:** Synthetic estimates must be within 25% of real-world industry averages

**FR-2.2: IT/Dev Reference Class Generation**
- **Requirement:** System shall generate synthetic reference classes for IT/development domain
- **Acceptance Criteria:**
  - 5 subcategories: api_development, mobile_app, web_app, integration, migration
  - Effort measured in person-weeks instead of cost
  - Complexity factors account for tech stack (simple framework vs custom)
  - Timeline distributions account for team size
- **Validation:** Brett will provide real project data for validation

**FR-2.3: Synthetic Data Validation & Tuning**
- **Requirement:** System shall compare synthetic reference classes to real feedback data and recommend adjustments
- **Acceptance Criteria:**
  - Calculate variance between synthetic P50/P80 and actual outcomes
  - Flag reference classes with >25% variance as "needs tuning"
  - Recommend adjustment percentages
  - Admin can approve/reject tuning recommendations
- **Performance:** Validation runs nightly for all reference classes with feedback

### FR-3: Multi-Tenant Infrastructure

**FR-3.1: Tenant Registration & Management**
- **Requirement:** System shall support tenant registration with email verification
- **Acceptance Criteria:**
  - Email verification required before activation
  - Generate unique API key per tenant
  - Support tier selection: Trial, Pro, Enterprise
  - Tenant settings: region preferences, branding, BYOK
- **Security:** Passwords hashed with bcrypt, API keys hashed for storage

**FR-3.2: Tenant Isolation**
- **Requirement:** System shall enforce hard tenant isolation at database and API level
- **Acceptance Criteria:**
  - All MongoDB queries include `tenant_id` filter
  - Zero cross-tenant data access possible
  - API responses only include tenant's own data
  - Rate limiting applied per tenant
- **Validation:** Security audit must confirm zero data leakage

**FR-3.3: BYOK (Bring Your Own Key)**
- **Requirement:** System shall securely store and use tenant-provided OpenAI API keys
- **Acceptance Criteria:**
  - Keys encrypted at rest with AES-256
  - Keys decrypted only at request time in memory
  - Keys never logged or transmitted in plaintext
  - Key validation on setup (test API call)
  - Fallback to platform key for Trial tier
- **Security:** Keys rotatable, audit log on key changes

**FR-3.4: Rate Limiting**
- **Requirement:** System shall enforce rate limits based on tenant tier
- **Acceptance Criteria:**
  - Trial: 10 requests/hour
  - Pro: 100 requests/hour
  - Enterprise: 1000 requests/hour
  - Return 429 Too Many Requests when exceeded
  - Include retry-after header
- **Performance:** Rate limit check adds <5ms latency

### FR-4: LLM Integration (OpenAI BYOK)

**FR-4.1: Estimate Narrative Generation**
- **Requirement:** System shall generate stakeholder-friendly estimate explanations using LLM
- **Acceptance Criteria:**
  - Narrative length: 150-300 words
  - Includes: similar projects, key assumptions, risk factors, next steps
  - Tone: professional, transparent, educational
  - Adapt for customer vs contractor audience
- **LLM:** GPT-4, temperature 0.7, max_tokens 500

**FR-4.2: Conversational Scoping (Chat)**
- **Requirement:** System shall conduct multi-turn conversation to gather project details
- **Acceptance Criteria:**
  - Ask clarifying questions based on missing attributes
  - Extract structured data from natural language
  - Determine when enough info exists for estimate (confidence >= 0.7)
  - Provide quick-select suggestions
  - Handle ambiguity gracefully
- **LLM:** GPT-4, structured output mode, function calling

**FR-4.3: BYOK Implementation**
- **Requirement:** System shall use tenant's OpenAI API key for all LLM requests
- **Acceptance Criteria:**
  - Decrypt key at request time
  - Include tenant_id in LLM request metadata
  - Handle API key errors gracefully (invalid, rate limited, quota exceeded)
  - Fallback to platform key for Trial tier only
- **Error Handling:** Return 402 Payment Required if tenant key quota exceeded

### FR-5: Feedback & Calibration System

**FR-5.1: Customer Feedback Submission**
- **Requirement:** System shall allow customers to submit actual project outcomes
- **Acceptance Criteria:**
  - Magic link sent via email (no login required)
  - Form fields: actual cost, actual timeline, rating (1-5), comments
  - Link expires after 90 days
  - Submission confirmation email to customer and tenant
- **UX:** Mobile-friendly form, <2 minutes to complete

**FR-5.2: Contractor Feedback Submission**
- **Requirement:** System shall allow contractors (tenants) to submit detailed actual breakdown
- **Acceptance Criteria:**
  - Requires tenant login
  - Form fields: actual cost breakdown, discrepancy explanation, flags (scope creep, hidden costs, market changes)
  - Can dispute customer feedback if variance >20%
  - Both feedbacks stored separately, flagged for review
- **Workflow:** If customer + contractor variance >10%, flag for admin review

**FR-5.3: Calibration Metrics Calculation**
- **Requirement:** System shall calculate accuracy metrics per tenant and reference class
- **Acceptance Criteria:**
  - Metrics: avg cost variance %, avg timeline variance %, % within 20%
  - Displayed per tenant and per reference class
  - Updated within 1 hour of feedback submission
  - Historical trend visible (improving/declining accuracy)
- **Performance:** Metric calculation runs async, results cached

**FR-5.4: Model Refinement from Feedback**
- **Requirement:** System shall use feedback to improve future estimates
- **Acceptance Criteria:**
  - LLM prompt includes recent feedback examples for similar projects
  - Reference class P50/P80 values adjusted if consistent bias detected (>5 submissions, >15% variance)
  - Admin approval required for reference class tuning
  - Audit log tracks all tuning changes
- **Validation:** After tuning, next 5 estimates should show improved accuracy

### FR-6: White Label Chat Widget

**FR-6.1: Widget Embedding**
- **Requirement:** System shall provide embeddable JavaScript widget for contractor websites
- **Acceptance Criteria:**
  - Embed code <5 lines of JavaScript
  - Widget loads asynchronously, doesn't block page load
  - Responsive design (works on mobile and desktop)
  - CORS and CSP compliant
- **Example Embed:**
  ```html
  <script src="https://widget.efofx.ai/embed.js"></script>
  <script>
    efofxWidget.init({ apiKey: 'efofx_...' });
  </script>
  ```

**FR-6.2: Widget Branding Customization**
- **Requirement:** System shall allow tenants to customize widget branding
- **Acceptance Criteria:**
  - Customizable: logo, primary color, button text, welcome message
  - Preview available in tenant dashboard
  - Changes apply to all widget instances within 5 minutes (CDN cache)
  - No efOfX branding visible to end customer
- **UX:** Color picker, logo upload, live preview

**FR-6.3: Widget Conversational UI**
- **Requirement:** Widget shall provide chat-based project scoping interface
- **Acceptance Criteria:**
  - Conversational flow: greet → gather details → generate estimate
  - Quick-select buttons for common options
  - Real-time typing indicators
  - Message history persists for 24 hours
  - Generates estimate when sufficient info gathered
- **Performance:** Message response time <2 seconds

**FR-6.4: Widget Lead Capture**
- **Requirement:** Widget shall capture customer contact information
- **Acceptance Criteria:**
  - Optional email/phone capture before showing estimate
  - Configurable: required vs optional
  - Data stored under tenant's account
  - Email notification to tenant on new lead
  - Optional: Send estimate PDF to customer email
- **Privacy:** GDPR/CCPA compliant, privacy policy link required

**FR-6.5: Widget Security**
- **Requirement:** Widget shall securely communicate with backend using tenant API key
- **Acceptance Criteria:**
  - API key never exposed in client-side code
  - Session tokens expire after 24 hours
  - Rate limiting per widget session (10 messages/session)
  - HTTPS only
  - XSS protection
- **Security:** Security audit required before launch

### FR-7: MCP Server (Reference Class Queries)

**FR-7.1: Reference Class Query Endpoint**
- **Requirement:** MCP function shall query reference classes by attributes
- **Acceptance Criteria:**
  - Filter by: category, subcategory, region, custom attributes
  - Return matched reference classes with confidence scores
  - Support partial matching and fuzzy search
  - Response time p95 <150ms (warm)
- **Caching:** LRU cache, 5min TTL

**FR-7.2: Regional Adjustment Application**
- **Requirement:** MCP function shall apply regional cost multipliers
- **Acceptance Criteria:**
  - Retrieve regional multiplier from reference class
  - Apply to baseline cost: adjusted = baseline × regional_multiplier
  - Support multiple regions per reference class
  - Return calculation breakdown
- **Performance:** <10ms calculation time

**FR-7.3: Performance Optimization**
- **Requirement:** MCP functions shall meet performance targets
- **Acceptance Criteria:**
  - Cold start <300ms
  - Warm requests p95 <150ms
  - LRU cache hit rate >80%
  - MongoDB connection pooling (module-scope singleton)
- **Monitoring:** Log all response times, alert if p95 >200ms

### FR-8: API Error Handling & Validation

**FR-8.1: Input Validation**
- **Requirement:** All API endpoints shall validate input using Pydantic models
- **Acceptance Criteria:**
  - Return 422 Unprocessable Entity for validation errors
  - Error response includes field name and specific error
  - String fields validated for length and format
  - Numeric fields validated for range
- **Example Error:**
  ```json
  {
    "error": "Validation failed",
    "details": [
      {"field": "description", "error": "Must be 10-2000 characters"}
    ]
  }
  ```

**FR-8.2: Error Response Standards**
- **Requirement:** All errors shall follow consistent response format
- **Acceptance Criteria:**
  - Status codes: 400 (bad request), 401 (unauthorized), 404 (not found), 422 (validation), 429 (rate limit), 500 (server error)
  - Response includes: error type, message, optional details
  - Request ID included for debugging
  - No sensitive data in error messages
- **Logging:** All errors logged with request ID

**FR-8.3: Rate Limit Handling**
- **Requirement:** API shall return clear error when rate limit exceeded
- **Acceptance Criteria:**
  - Status 429 Too Many Requests
  - Include `Retry-After` header (seconds)
  - Include current limit and reset time
  - Suggest upgrading tier if applicable
- **Example Response:**
  ```json
  {
    "error": "Rate limit exceeded",
    "message": "You've made 101 requests in the last hour (limit: 100)",
    "retry_after": 3420,
    "upgrade_url": "/pricing"
  }
  ```

---

## Non-Functional Requirements

### Performance

**NFR-P1: API Response Times**
- FastAPI endpoints p95 < 500ms (excluding LLM calls)
- LLM-powered endpoints p95 < 3 seconds
- MCP functions p95 < 150ms (warm), cold start < 300ms
- Database queries p95 < 50ms
- **Rationale:** User-facing estimation requests must feel responsive, especially in widget context

**NFR-P2: Throughput & Concurrency**
- Support 100 concurrent estimation requests per tenant (Pro tier)
- Support 1000 concurrent estimation requests across platform
- Widget sessions: 50 concurrent conversations per tenant
- **Rationale:** Contractors may embed widget on high-traffic sites

**NFR-P3: LLM Performance**
- OpenAI API calls complete within 2-5 seconds
- Retry failed LLM calls with exponential backoff (max 3 retries)
- Cache LLM responses for identical inputs (1 hour TTL)
- **Rationale:** LLM calls are the slowest component, must be optimized

**NFR-P4: Database Performance**
- MongoDB connection pool: 10-50 connections
- Index coverage for all tenant-scoped queries
- Compound indexes on (tenant_id, category, region)
- Query plan analysis for slow queries (>100ms)
- **Rationale:** Multi-tenant queries must be fast despite large datasets

### Security

**NFR-S1: Data Encryption**
- All data encrypted in transit (TLS 1.3)
- Tenant OpenAI keys encrypted at rest (AES-256)
- Passwords hashed with bcrypt (cost factor 12)
- API keys hashed for storage, plaintext never retrievable
- **Compliance:** Required for SOC 2 compliance (future)

**NFR-S2: Authentication & Authorization**
- JWT tokens expire after 24 hours
- Refresh tokens expire after 30 days
- Rate limit failed login attempts (5 per 15 minutes)
- MFA support for Enterprise tier tenants
- **Rationale:** Prevent unauthorized access and brute force attacks

**NFR-S3: Tenant Isolation (Zero Trust)**
- 100% of database queries include tenant_id filter
- API endpoints validate tenant ownership before operations
- No shared resources between tenants (reference classes excepted)
- Security audit confirms zero cross-tenant data leakage
- **Compliance:** Critical for multi-tenant SaaS trust

**NFR-S4: API Security**
- CORS configured for known origins only
- CSRF protection on state-changing operations
- Input sanitization prevents SQL/NoSQL injection
- Rate limiting prevents DDoS (per-tenant and global)
- **Rationale:** Publicly accessible API must resist attacks

**NFR-S5: Widget Security**
- Widget API key never exposed in client code (proxy through session tokens)
- XSS protection via Content Security Policy (CSP)
- Session tokens expire after 24 hours
- HTTPS required for all widget communications
- **Rationale:** Widget is end-customer facing, highest attack surface

### Scalability

**NFR-SC1: Horizontal Scaling**
- FastAPI backend stateless, scales horizontally behind load balancer
- MCP functions auto-scale based on demand (serverless)
- MongoDB Atlas auto-scaling for storage and compute
- No session affinity required
- **Rationale:** Must handle tenant growth without architecture changes

**NFR-SC2: Data Growth**
- Support 100,000 estimates per month across all tenants
- Support 10,000 reference classes (platform + custom)
- Support 50,000 feedback submissions per month
- MongoDB sharding strategy planned for >1TB data
- **Rationale:** Growth projections for year 1

**NFR-SC3: Multi-Region Support (Future)**
- Architecture supports multi-region deployment
- Reference classes can be region-specific
- Low latency (<200ms) for users in US West Coast (MVP)
- **Rationale:** Construction domain is region-specific, architecture must support it

### Reliability & Availability

**NFR-R1: Uptime**
- Target 99.5% uptime (43.8 hours downtime/year allowed)
- Graceful degradation if LLM unavailable (return estimates without narrative)
- Graceful degradation if MCP functions unavailable (fallback to direct MongoDB)
- **Rationale:** MVP SLA, not mission-critical yet

**NFR-R2: Data Durability**
- MongoDB Atlas automatic backups (point-in-time recovery)
- Backup retention: 7 days
- Disaster recovery plan documented
- **Rationale:** Customer estimation data must not be lost

**NFR-R3: Error Recovery**
- Failed estimate requests return helpful error messages
- Partial failures (e.g., LLM timeout) return best-effort estimate
- Retry logic for transient failures (MongoDB connection, LLM API)
- **Rationale:** User experience should not break on transient errors

### Monitoring & Observability

**NFR-M1: Logging**
- Structured JSON logs with request_id, tenant_id, timestamp
- Log levels: DEBUG, INFO, WARNING, ERROR
- Retain logs for 30 days (compliance requirement)
- **Tools:** DigitalOcean Functions logs, MongoDB Atlas logs

**NFR-M2: Metrics & Alerting**
- Track: request rate, error rate, response time (p50/p95/p99)
- Alert on: error rate >5%, response time p95 >1s, rate limit hits
- Dashboard shows: tenant activity, reference class usage, feedback rate
- **Tools:** DigitalOcean monitoring, custom metrics API

**NFR-M3: Audit Logging**
- Log all tenant configuration changes
- Log all reference class tuning approvals
- Log all BYOK key changes
- Audit logs retained for 1 year
- **Compliance:** Required for SOC 2, GDPR

### Maintainability

**NFR-MA1: Code Quality**
- Python: Black formatting, Flake8 linting, type hints (mypy)
- JavaScript: ESLint, Prettier formatting
- Test coverage >70% for critical paths (RCF engine, auth)
- **Rationale:** Brownfield project needs standards for additions

**NFR-MA2: Documentation**
- OpenAPI (Swagger) docs auto-generated from FastAPI
- API endpoint documentation includes examples
- Data model documentation auto-generated from Pydantic
- Widget integration guide with working examples
- **Rationale:** External tenants need clear integration docs

**NFR-MA3: Deployment**
- Zero-downtime deployments (rolling updates)
- Database migrations applied automatically (pre-deployment)
- Rollback plan for failed deployments
- Staging environment mirrors production
- **Rationale:** Minimize deployment risk

### Usability (Widget Specific)

**NFR-U1: Widget Loading Performance**
- Widget JavaScript bundle <50KB gzipped
- Initial render <1 second on 3G connection
- Lazy load assets (images, fonts)
- **Rationale:** Customer-facing widget on contractor sites

**NFR-U2: Mobile Responsiveness**
- Widget works on mobile devices (iOS/Android)
- Touch-friendly buttons (min 44x44px)
- Keyboard input optimized for mobile
- **Rationale:** 60%+ of contractor website traffic is mobile

**NFR-U3: Accessibility (Basic)**
- WCAG 2.1 Level A compliance (widget)
- Keyboard navigation supported
- Screen reader compatible (ARIA labels)
- **Rationale:** Basic accessibility, not mission-critical for MVP

### Integration

**NFR-I1: LLM Provider Integration**
- OpenAI API: GPT-4 via REST API
- Support structured outputs (JSON mode)
- Handle rate limiting gracefully (429 responses)
- **Future:** Architecture supports swapping LLM providers

**NFR-I2: Email Integration**
- Transactional email for: signup, feedback links, notifications
- Email service: SendGrid or similar
- Template management for branded emails
- **Rationale:** Critical for feedback loop

**NFR-I3: Analytics Integration (Future)**
- Architecture supports event tracking (Mixpanel, Segment)
- Track: widget interactions, estimate conversions, feedback submissions
- **MVP:** Basic logging only, dedicated analytics post-MVP

---

## Implementation Planning

### Development Approach

**Brownfield Strategy:**
- Leverage existing FastAPI backend, MongoDB integration, MCP functions
- Enhance existing RCF engine (already functional)
- Add new features: synthetic data generation, feedback system, white label widget
- Refactor for domain-agnosticism (remove construction hardcoding)

**Phase Sequencing:**

**Phase 1: Foundation (Weeks 1-3)**
- Synthetic data generation for construction domain
- Domain-agnostic backend refactoring
- Enhanced RCF engine with P50/P80 distributions

**Phase 2: Feedback Loop (Weeks 4-5)**
- Customer feedback form with magic links
- Contractor feedback submission
- Calibration metrics calculation
- Model refinement from feedback

**Phase 3: White Label Widget (Weeks 6-8)**
- Widget JavaScript SDK
- Branding customization
- Conversational UI
- Lead capture integration

**Phase 4: IT/Dev Domain (Weeks 9-10)**
- IT/dev reference class generation
- Validation with real project data (Brett)
- Cross-domain testing

**Epic Breakdown Required:**
Requirements must be decomposed into epics and bite-sized stories for 200k context dev agents.

**Next Step:** Run `/bmad:bmm:workflows:create-epics-and-stories` to create the implementation breakdown.

---

## PRD Summary

### What We've Defined

efOfX Estimation Service MVP delivers an AI-powered, multi-tenant estimation platform with three breakthrough capabilities:

1. **Synthetic Data Generation** - Bootstraps the platform with realistic reference classes for construction (7 types) and IT/dev (5 types) domains without requiring extensive historical data

2. **Feedback-Driven Calibration** - Closes the learning loop with dual feedback (customer + contractor), calculates accuracy metrics, and tunes estimates based on real outcomes

3. **White Label Distribution** - Embeddable chat widget (<5 lines of code) enables contractors to offer estimation on their websites with full branding customization

### Core Requirements Count

- **Functional Requirements:** 8 major feature areas, 25 detailed requirements
- **Non-Functional Requirements:** 7 categories (performance, security, scalability, reliability, monitoring, maintainability, integration)
- **API Endpoints:** 15 endpoints across 6 categories
- **Data Models:** 5 MongoDB collections with domain-agnostic schema

### What Makes This Special

The product magic woven throughout this PRD:

**"A communication coach that happens to estimate projects"** - efOfX doesn't just predict costs, it teaches transparent communication through:
- P50/P80 distributions instead of false precision
- Visible assumptions and risks
- Calibration metrics that build trust over time
- Contractor and customer feedback reconciliation

**Trust through transparency** - The competitive moat is earned, not built:
- Self-improving estimates (feedback loop)
- Domain-agnostic architecture (proven across construction + IT)
- BYOK model respects tenant data ownership

### Success Criteria Recap

MVP succeeds when:
- ✅ **70% of estimates within 20%** of actual outcomes (construction)
- ✅ **40% feedback submission rate** (closes the loop)
- ✅ **10 contractors embed widget** (distribution)
- ✅ **15 active BYOK tenants** (business viability)
- ✅ **2 domains validated** (construction + IT/dev)

---

## References

**Input Documents:**
- Brainstorming Session: [docs/bmm-brainstorming-session-2025-11-09.md](docs/bmm-brainstorming-session-2025-11-09.md)
- Project Documentation: [docs/index.md](docs/index.md)
- Existing Architecture: [docs/architecture.md](docs/architecture.md)
- Existing API Contracts: [docs/api-contracts-efofx-estimate.md](docs/api-contracts-efofx-estimate.md)
- Existing Data Models: [docs/data-models-efofx-estimate.md](docs/data-models-efofx-estimate.md)

**Strategic Context:**
- Product vision: "Communication coach that happens to estimate projects"
- Core principle: "The performance layer isn't a mask—it's a mirror"
- Competitive strategy: Trust moat > Feature moat
- Distribution strategy: White label widget > Direct sales

---

## Next Steps

### Immediate: Epic & Story Breakdown

Run: `/bmad:bmm:workflows:create-epics-and-stories`

This will decompose the PRD into:
- Implementation epics (logical groupings)
- Bite-sized user stories (< 200k context for dev agents)
- Acceptance criteria per story
- Epic sequencing based on dependencies

### Recommended Follow-Ups

1. **UX Design (Widget)** - Run: `/bmad:bmm:workflows:create-ux-design`
   - White label chat widget UI/UX
   - Feedback form design
   - Tenant dashboard wireframes

2. **Architecture Refinement** - Run: `/bmad:bmm:workflows:create-architecture`
   - Widget JavaScript SDK architecture
   - Synthetic data generation pipeline
   - Feedback calibration system design

3. **Technical Specification** - Run: `/bmad:bmm:workflows:epic-tech-context`
   - Per-epic technical design decisions
   - Integration patterns
   - Data migration strategy (refactoring for domain-agnostic)

---

## Product Magic Essence

**efOfX transforms project estimation from guesswork into transparent, trust-building communication.**

Instead of:
- ❌ Single point estimates that are always wrong
- ❌ Padding estimates to avoid difficult conversations
- ❌ Estimates that never learn from reality

efOfX delivers:
- ✅ **P50/P80 ranges** that teach stakeholders to think probabilistically
- ✅ **Visible assumptions and risks** that enable informed decisions
- ✅ **Self-improving accuracy** through closed feedback loops
- ✅ **Communication coaching** that helps contractors explain complexity

**The magic:** Every estimate becomes a teachable moment. The system doesn't just predict—it coaches users on how to communicate uncertainty effectively.

This isn't another estimation tool. It's a **trust-building platform** that makes transparent communication the competitive advantage.

---

_This PRD captures the full vision of efOfX Estimation Service MVP - from synthetic data bootstrapping to white label distribution - all built on the foundation of trust through transparency._

_Created through collaborative discovery between Brett and AI facilitator (Mary, Business Analyst)._
_Date: 2025-11-09_
