# Efofx v1 Product Requirements Document (PRD)

## 1. Overview

Efofx is a project intake and estimation system designed to transform vague, unstructured customer requests into structured project scopes and realistic estimate ranges.

The core value proposition is simple:

**Efofx helps businesses qualify project leads faster by turning rough customer ideas into structured scopes and realistic estimate ranges.**

This MVP focuses on validating:
- Whether users trust AI-assisted structured intake
- Whether businesses find leads more usable
- Whether estimate ranges improve early-stage conversations

---

## 2. Problem Statement

Most project-based businesses suffer from poor initial intake quality.

Customers typically provide:
- incomplete requirements
- inconsistent terminology
- unrealistic expectations
- missing constraints

This leads to:
- wasted time on discovery
- inconsistent estimates
- poor lead quality
- slow response times

Traditional solutions rely on rigid forms, which are:
- brittle
- incomplete
- difficult to maintain

Generic AI tools (e.g., ChatGPT) provide answers but lack:
- structure
- domain-specific logic
- explainability
- feedback loops

---

## 3. Product Vision (MVP Scope)

Efofx v1 is a **structured project intake and estimation assistant**.

It performs four core functions:

1. Accept plain-language project descriptions
2. Ask intelligent follow-up questions
3. Generate structured project summaries
4. Produce rough estimate ranges with assumptions and confidence

---

## 4. Target Users

### Primary (Recommended)
**Service marketplaces / contractor directories**

Why:
- Need better lead qualification
- Benefit from structured routing
- Higher willingness to adopt tooling

### Secondary
**Individual contractors / service providers**

Risks:
- Lower trust in automation
- Concern about losing leads
- Lower technical adoption

---

## 5. MVP User Flow

### Step 1: Project Input
User enters a plain-language description:

> "Tell us what you want to build, fix, or improve."

---

### Step 2: Dynamic Follow-Up Questions
System asks 3–7 targeted questions based on project type.

Example (Pool):
- Desired size range
- Yard conditions (flat/sloped)
- Special features (slide, spa, waterfall)
- Equipment access
- Location (ZIP/city)

Goal: Gather sufficient detail for estimation, not perfect completeness.

---

### Step 3: Structured Project Summary
System generates:
- Project type
- Key features
- Location
- Constraints
- Assumptions
- Missing unknowns

---

### Step 4: Estimate Output
System provides:

**Estimated Range**
- $X to $Y

**Confidence**
- Low / Medium / High

**Based On**
- Comparable project patterns
- Selected features
- Regional adjustments

**Assumptions**
- Explicit bullet points

**Next Step**
- Connect with provider or request formal quote

---

### Step 5: Lead Capture
Capture:
- Name
- Email
- Optional phone

Two test variants:
- Soft gate after summary
- Hard gate before estimate

---

## 6. Core Features (MVP)

### 6.1 Dynamic Intake Engine
- Context-aware follow-up questions
- Domain-specific prompts

### 6.2 Project Normalization
- Convert conversation into structured schema
- Extract attributes and tags

### 6.3 Estimation Engine
- Generate range based on reference-class logic
- Apply feature-based adjustments
- Apply regional multipliers

### 6.4 Explainability Layer
- Confidence levels
- Assumptions
- Attribution (data vs heuristic)

### 6.5 Lead Dashboard (Basic)
- View leads
- View summaries
- View estimates
- Export or forward leads

### 6.6 Domain Configuration
- Start with ONE vertical (recommended: outdoor projects)

---

## 7. Non-Goals (Out of Scope for MVP)

- Bring-your-own API keys
- Multi-model support
- Advanced billing systems
- Complex analytics dashboards
- Automated feedback tuning jobs
- Multi-industry support
- Plugin ecosystem
- Deep CRM functionality
- Full proposal generation

---

## 8. Data Model (MVP)

Each lead should store:

- Tenant ID
- Project type
- Raw project description
- Structured attributes
- Location
- Estimate low
- Estimate high
- Confidence level
- Assumptions
- Contact info
- Conversation transcript
- Lead status
- Optional outcome feedback (future)

---

## 9. Output Design

### User-Facing
- Project summary
- Estimate range
- Assumptions
- Confidence level
- Recommended next step

### Business-Facing
- Structured metadata
- Lead details
- Estimate results
- Conversation history

---

## 10. Pricing Strategy (MVP)

### Option 1: Free Pilot
- 30-day trial
- Usage cap

### Option 2: Flat Fee
- $99–$299/month
- Includes hosting and usage limits

### Option 3: Partner Pilot
- Integrated with marketplace or agency
- Revenue share or co-sell model

Goal: Validate value, not maximize revenue

---

## 11. Success Metrics

### User Metrics
- Intake completion rate
- Lead capture rate
- Conversation length
- Estimate view rate

### Business Metrics
- Lead usability rate
- Time saved in discovery
- Operator trust feedback
- Retention after pilot

### Model Metrics
- Estimate accuracy (sampled)
- Confidence calibration
- Failure pattern tracking

---

## 12. Differentiation

Efofx is not a chatbot.

It is:
- A structured intake system
- An estimation engine
- A feedback-driven improvement loop

Key distinction:

**Chatbots answer questions. Efofx builds estimation-ready project scopes.**

---

## 13. Initial Vertical Recommendation

### Primary: Outdoor Projects
- Pools
- Landscaping
- Patios
- Retaining walls

Why:
- High ticket
- High variability
- Strong need for early estimation

### Secondary: Kitchens and Bathrooms

---

## 14. Technical Approach (High-Level)

- Frontend: Embedded intake widget
- Backend: FastAPI service
- Database: Document-based (MongoDB)
- LLM: OpenAI (initially)
- Estimation: Reference-class + heuristic adjustments

---

## 15. Risks and Mitigations

### Risk: "Just another chatbot"
Mitigation:
- Lead with structured output and estimates
- Avoid chatbot-first positioning

### Risk: Trust in estimates
Mitigation:
- Show assumptions and confidence
- Make reasoning transparent

### Risk: Cost unpredictability
Mitigation:
- Platform-owned API key
- Usage caps per tenant

### Risk: Low adoption by contractors
Mitigation:
- Target marketplaces first

---

## 16. Next Steps

1. Build a single polished demo flow (one vertical)
2. Launch landing page focused on estimation value
3. Develop basic lead dashboard
4. Pilot with one partner (marketplace preferred)
5. Collect qualitative feedback on:
   - trust
   - usability
   - lead quality

---

## 17. Product Promise

**Efofx helps businesses turn vague project requests into structured, estimate-ready leads in minutes.**

This is the core of the MVP and the foundation for future expansion.

