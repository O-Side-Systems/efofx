# Functional Requirements

## FR-1: Reference Class Forecasting Engine

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

## FR-2: Synthetic Data Generation

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

## FR-3: Multi-Tenant Infrastructure

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

## FR-4: LLM Integration (OpenAI BYOK)

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

## FR-5: Feedback & Calibration System

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

## FR-6: White Label Chat Widget

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

## FR-7: MCP Server (Reference Class Queries)

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

## FR-8: API Error Handling & Validation

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
