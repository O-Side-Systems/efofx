# Product Scope

## MVP - Minimum Viable Product

**Goal:** Prove the core value proposition with construction domain, establish technical foundation for multi-domain expansion.

### Core Estimation Engine (Foundational RCF)
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

### Multi-Tenant Infrastructure
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

### LLM Integration (OpenAI BYOK)
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

### Feedback & Calibration System
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

### White Label Chat Widget (Distribution)
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

### MCP Server (Reference Class Queries)
- **Serverless Functions** (Existing, enhancements)
  - Query reference classes by attributes
  - Retrieve single reference class by ID
  - Apply regional/complexity adjustments
  - Cached responses (5min TTL)

- **Performance**
  - p95 response time < 150ms (warm)
  - Cold start < 300ms
  - LRU caching for popular queries

### API Endpoints (FastAPI Backend)
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

## Growth Features (Post-MVP)

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

## Vision (Future)

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
