# efOfX Architecture Diagrams

This document provides visual representations of the efOfX AI-Powered Estimation Platform architecture using mermaid diagrams.

---

## Table of Contents
1. [System Overview](#system-overview)
2. [Multi-Tenant Architecture](#multi-tenant-architecture)
3. [Authentication Flow](#authentication-flow)
4. [White Label Widget Integration](#white-label-widget-integration)
5. [Estimation Request Flow](#estimation-request-flow)
6. [Feedback Loop & Calibration](#feedback-loop--calibration)
7. [Data Model](#data-model)
8. [Deployment Architecture](#deployment-architecture)
9. [Domain-Agnostic Design](#domain-agnostic-design)

---

## System Overview

High-level system architecture showing all major components and their relationships.

```mermaid
graph TB
    subgraph "Client Layer"
        Widget[White Label Chat Widget<br/>React + TypeScript]
        Admin[Tenant Admin UI<br/>Future]
    end
    
    subgraph "API Layer - FastAPI"
        Gateway[API Gateway<br/>JWT Auth]
        EstAPI[Estimate Service]
        ChatAPI[Chat Service]
        FeedbackAPI[Feedback Service]
        WidgetAPI[Widget Config Service]
        TenantAPI[Tenant Service]
    end
    
    subgraph "Business Logic"
        RCF[Reference Class<br/>Forecasting Engine]
        Matching[Project Matching<br/>Algorithm]
        LLM[LLM Integration<br/>GPT-4o-mini]
        Prompts[Git-Based Prompt<br/>Management]
    end
    
    subgraph "Data Layer"
        MongoDB[(MongoDB Atlas<br/>Multi-Tenant)]
        Spaces[DigitalOcean Spaces<br/>S3-Compatible Storage]
    end
    
    subgraph "Infrastructure"
        CDN[DO Spaces CDN<br/>274 Global PoPs]
        Email[SendGrid<br/>Email Service]
        Monitor[Sentry + DO Metrics<br/>Monitoring]
    end
    
    Widget --> Gateway
    Admin -.-> Gateway
    Gateway --> EstAPI
    Gateway --> ChatAPI
    Gateway --> FeedbackAPI
    Gateway --> WidgetAPI
    Gateway --> TenantAPI
    
    EstAPI --> RCF
    ChatAPI --> LLM
    RCF --> Matching
    EstAPI --> LLM
    LLM --> Prompts
    
    EstAPI --> MongoDB
    FeedbackAPI --> MongoDB
    WidgetAPI --> MongoDB
    TenantAPI --> MongoDB
    Matching --> MongoDB
    
    WidgetAPI --> Spaces
    Widget --> CDN
    CDN --> Spaces
    
    FeedbackAPI --> Email
    EstAPI --> Monitor
    
    style Widget fill:#e1f5ff
    style MongoDB fill:#47a248
    style LLM fill:#ff6b6b
    style CDN fill:#0080ff
```

---

## Multi-Tenant Architecture

Zero-trust multi-tenant isolation pattern with BYOK (Bring Your Own Key) for OpenAI.

```mermaid
graph TB
    subgraph "Request Flow"
        Request[Incoming Request]
        JWT[JWT Token<br/>tenant_id: ABC]
    end
    
    subgraph "Tenant Isolation Layer"
        Deps[FastAPI Dependency<br/>Injection]
        TenantID[Extract tenant_id<br/>from JWT]
    end
    
    subgraph "Service Layer"
        Service[Service Method<br/>tenant_id as first param]
    end
    
    subgraph "Database Layer - MongoDB"
        Query[Query with tenant_id filter<br/>MANDATORY]
        Index[Index: tenant_id, category, region]
        
        subgraph "Collections"
            T1[(Tenant ABC Data)]
            T2[(Tenant XYZ Data)]
            T3[(Tenant 123 Data)]
        end
    end
    
    subgraph "Encryption Layer"
        BYOK[Encrypted OpenAI Keys<br/>AES-256]
        Decrypt[Decrypt at Request Time]
    end
    
    Request --> JWT
    JWT --> Deps
    Deps --> TenantID
    TenantID --> Service
    Service --> Query
    Query --> Index
    Index --> T1
    
    Service --> BYOK
    BYOK --> Decrypt
    Decrypt --> LLM[LLM Request]
    
    style JWT fill:#ffd700
    style TenantID fill:#ff6b6b
    style Query fill:#ff4444
    style BYOK fill:#9b59b6
```

---

## Authentication Flow

Three-layer authentication system for different actors in the system.

```mermaid
sequenceDiagram
    participant User as Tenant Admin/User
    participant Widget as Chat Widget
    participant API as FastAPI Backend
    participant MCP as MCP Serverless
    participant DB as MongoDB
    
    Note over User,MCP: Layer 1: User → API (JWT)
    User->>API: Login with credentials
    API->>DB: Verify user & tenant
    DB-->>API: User record
    API->>API: Generate JWT<br/>(tenant_id, role, exp)
    API-->>User: JWT Bearer Token
    User->>API: Request with JWT
    API->>API: Validate JWT & extract tenant_id
    API-->>User: Response
    
    Note over Widget,MCP: Layer 2: Widget → API (Session Token)
    Widget->>API: Initialize widget session
    API->>API: Generate session token
    API-->>Widget: Session token
    Widget->>API: Estimate request + session token
    API->>API: Validate session
    API-->>Widget: Estimate response
    
    Note over API,MCP: Layer 3: API → MCP (HMAC + JWT)
    API->>API: Generate HMAC signature
    API->>MCP: Request with HMAC + JWT
    MCP->>MCP: Verify HMAC & JWT
    MCP-->>API: Response
```

---

## White Label Widget Integration

How contractors/agencies embed the widget on their websites.

```mermaid
graph LR
    subgraph "Partner Website"
        Page[Partner HTML Page<br/>contractor.com]
        Embed[5-Line Embed Code]
    end
    
    subgraph "Widget Loading"
        CDN[DO Spaces CDN<br/>embed.js]
        Shadow[Shadow DOM<br/>Style Isolation]
        Widget[React Widget<br/>Chat Interface]
    end
    
    subgraph "Configuration"
        API[Widget Config API]
        Branding[Tenant Branding<br/>logo, colors, name]
    end
    
    subgraph "Backend"
        Chat[Chat Service]
        Est[Estimate Service]
    end
    
    Page --> Embed
    Embed --> CDN
    CDN --> Shadow
    Shadow --> Widget
    Widget --> API
    API --> Branding
    Branding --> Widget
    Widget --> Chat
    Widget --> Est
    
    style Embed fill:#4CAF50
    style CDN fill:#0080ff
    style Shadow fill:#9c27b0
```

**Embed Code Example:**
```html
<div id="efofx-widget"></div>
<script src="https://widget.efofx.ai/embed.js"></script>
<script>
  efofxWidget.init({
    apiKey: 'efofx_...',
    primaryColor: '#2563eb',
    logo: 'https://partner.com/logo.png',
    companyName: 'ACME Construction'
  });
</script>
```

---

## Estimation Request Flow

Complete flow from user question to estimate with reference class matching.

```mermaid
sequenceDiagram
    participant U as User (via Widget)
    participant Chat as Chat Service
    participant LLM as GPT-4o-mini
    participant Match as Matching Algorithm
    participant RC as Reference Classes<br/>(MongoDB)
    participant Est as Estimate Service
    participant Prompts as Prompt Config<br/>(Git JSON)
    
    U->>Chat: "I need to remodel my kitchen"
    Chat->>LLM: Scope clarification prompt
    LLM-->>Chat: Questions about size, materials, etc.
    Chat-->>U: Interactive questions
    
    U->>Chat: Provides project details
    Chat->>LLM: Extract structured data
    LLM-->>Chat: {category: "construction",<br/>subcategory: "kitchen_remodel",<br/>attributes: {...}}
    
    Chat->>Match: Find similar projects
    Match->>RC: Query reference classes<br/>tenant_id + category + attributes
    RC-->>Match: Top 10 similar projects<br/>(similarity scores)
    Match-->>Chat: Matched reference classes
    
    Chat->>Est: Generate estimate
    Est->>Est: Calculate P50, P80, P90<br/>from reference class distributions
    Est->>Prompts: Load narrative template<br/>(domain-specific)
    Prompts-->>Est: Template with domain context
    
    Est->>LLM: Generate narrative<br/>(P50/P80 + assumptions)
    LLM-->>Est: Human-readable explanation
    
    Est-->>U: Estimate with:<br/>- P50/P80/P90 bands<br/>- Narrative<br/>- Reference projects<br/>- Assumptions<br/>- Confidence score
    
    Note over U,Est: Store estimate_id + prompt_version<br/>for calibration tracking
```

---

## Feedback Loop & Calibration

Self-improving system through actual project outcome tracking.

```mermaid
graph TB
    subgraph "Estimation Phase"
        E1[User requests estimate]
        E2[Generate estimate<br/>with prompt_version]
        E3[Store estimate_id<br/>in database]
    end
    
    subgraph "Project Execution"
        P1[Project completes]
        P2[User submits actual outcome<br/>via feedback form]
    end
    
    subgraph "Feedback Collection"
        F1[Feedback API]
        F2[Match to estimate_id]
        F3[Store actual vs estimated]
        F4[Calculate accuracy<br/>variance, bias]
    end
    
    subgraph "Calibration Engine"
        C1[Analyze by reference class]
        C2[Compute calibration scores<br/>per domain/region]
        C3[Identify systematic biases]
        C4[Generate tuning insights]
    end
    
    subgraph "System Improvement"
        I1[Adjust distribution parameters]
        I2[Update prompt templates]
        I3[Add actual project<br/>to reference classes]
        I4[Display calibration metrics<br/>to users]
    end
    
    E1 --> E2 --> E3
    E3 --> P1
    P1 --> P2
    P2 --> F1
    F1 --> F2
    F2 --> F3
    F3 --> F4
    F4 --> C1
    C1 --> C2
    C2 --> C3
    C3 --> C4
    C4 --> I1
    C4 --> I2
    C4 --> I3
    C4 --> I4
    
    I1 --> E2
    I2 --> E2
    I3 --> E2
    
    style E2 fill:#4CAF50
    style F3 fill:#ff9800
    style C4 fill:#2196F3
    style I3 fill:#9c27b0
```

**Calibration Metrics Displayed to Users:**
- "73% of estimates within 10% for similar projects"
- "Typical variance: +12% for kitchen remodels in SoCal"
- "Truth Integrity Score: 92%"

---

## Data Model

Core MongoDB collections with tenant isolation.

```mermaid
erDiagram
    TENANTS ||--o{ REFERENCE_CLASSES : owns
    TENANTS ||--o{ ESTIMATES : owns
    ESTIMATES ||--o{ FEEDBACK : tracks
    REFERENCE_CLASSES ||--o{ ESTIMATES : matches
    
    TENANTS {
        ObjectId _id PK
        string tenant_id UK
        string company_name
        string openai_api_key_encrypted "AES-256"
        object branding "logo, colors"
        datetime created_at
    }
    
    REFERENCE_CLASSES {
        ObjectId _id PK
        string tenant_id FK "MANDATORY in all queries"
        string category "construction, it_dev, etc"
        string subcategory "kitchen_remodel, api_dev"
        string region "socal, northeast, remote"
        object attributes "flexible schema"
        object cost_distribution "currency, p50, p80, p90"
        object timeline_distribution "unit, p50, p80, p90"
        string data_source "synthetic, actual, hybrid"
        datetime created_at
    }
    
    ESTIMATES {
        ObjectId _id PK
        string estimate_id UK
        string tenant_id FK "MANDATORY"
        string session_id
        object project_details
        array matched_reference_classes
        object estimate_bands "p50, p80, p90"
        string narrative_text "LLM-generated"
        string prompt_version "for calibration"
        array assumptions
        float confidence_score
        datetime created_at
    }
    
    FEEDBACK {
        ObjectId _id PK
        string tenant_id FK "MANDATORY"
        string estimate_id FK
        object actual_outcome "cost, timeline"
        float variance_percentage
        string completion_status
        text learnings
        datetime submitted_at
    }
```

**Critical Indexes:**
```javascript
// ALL indexes include tenant_id first for isolation
db.reference_classes.createIndex({ tenant_id: 1, category: 1, region: 1 })
db.estimates.createIndex({ tenant_id: 1, created_at: -1 })
db.estimates.createIndex({ tenant_id: 1, estimate_id: 1 })
db.feedback.createIndex({ tenant_id: 1, estimate_id: 1 })
```

---

## Deployment Architecture

DigitalOcean App Platform with auto-deploy and zero-downtime rolling deployments.

```mermaid
graph TB
    subgraph "Development"
        Dev[Developer]
        Git[GitHub Repository<br/>main branch]
    end
    
    subgraph "Build Pipeline"
        DO[DigitalOcean<br/>Auto-Deploy]
        Docker[Build Docker Image<br/>2-3 minutes]
    end
    
    subgraph "Production Environment"
        LB[Load Balancer]
        
        subgraph "App Instances"
            I1[Instance 1<br/>New Version]
            I2[Instance 2<br/>Old Version]
            I3[Instance 3<br/>New Version]
        end
        
        Drain[Drain Old Instances<br/>30 seconds]
    end
    
    subgraph "Infrastructure"
        Mongo[(MongoDB Atlas<br/>Managed)]
        Spaces[DO Spaces<br/>CDN + Storage]
        Sentry[Sentry<br/>Error Tracking]
    end
    
    subgraph "Configuration"
        Env[Environment Variables<br/>.do/app.yaml]
        Secrets[Encrypted Secrets<br/>JWT_SECRET, ENCRYPTION_KEY]
    end
    
    Dev --> Git
    Git --> DO
    DO --> Docker
    Docker --> LB
    LB --> I1
    LB --> I2
    LB --> I3
    I2 --> Drain
    
    I1 --> Mongo
    I1 --> Spaces
    I1 --> Sentry
    
    Env --> I1
    Secrets --> I1
    
    style Git fill:#4CAF50
    style Docker fill:#2196F3
    style LB fill:#ff9800
    style Mongo fill:#47a248
```

**Deployment Flow:**
1. `git push origin main`
2. DO detects push
3. Build Docker image (2-3 min)
4. Zero-downtime rolling deployment (30 sec)
5. Old instances drained, new instances live

**Monitoring:**
- DO App Platform: CPU, memory, restarts
- Sentry: Error tracking + performance tracing
- Both FREE for MVP

---

## Domain-Agnostic Design

How the same engine supports construction, IT/dev, and future domains.

```mermaid
graph TB
    subgraph "User Input"
        U1[Construction:<br/>"Remodel kitchen"]
        U2[IT/Dev:<br/>"Build REST API"]
        U3[Future:<br/>"Plan marketing campaign"]
    end
    
    subgraph "LLM Parsing"
        LLM[GPT-4o-mini<br/>Domain Detection]
    end
    
    subgraph "Domain-Specific Context"
        P1[Prompt Config:<br/>construction]
        P2[Prompt Config:<br/>it_dev]
        P3[Prompt Config:<br/>marketing]
    end
    
    subgraph "Domain-Agnostic Core"
        Match[Matching Algorithm<br/>Generic]
        RCF[RCF Engine<br/>Generic]
    end
    
    subgraph "Reference Classes by Domain"
        RC1[(Construction:<br/>kitchen_remodel<br/>cost: USD<br/>timeline: weeks)]
        RC2[(IT/Dev:<br/>api_development<br/>cost: person-weeks<br/>timeline: sprints)]
        RC3[(Marketing:<br/>campaign_launch<br/>cost: USD<br/>timeline: months)]
    end
    
    subgraph "Output"
        O1[Domain-Specific<br/>Narrative]
    end
    
    U1 --> LLM
    U2 --> LLM
    U3 --> LLM
    
    LLM --> P1
    LLM --> P2
    LLM --> P3
    
    P1 --> Match
    P2 --> Match
    P3 --> Match
    
    Match --> RC1
    Match --> RC2
    Match --> RC3
    
    RC1 --> RCF
    RC2 --> RCF
    RC3 --> RCF
    
    RCF --> O1
    
    style LLM fill:#ff6b6b
    style Match fill:#4CAF50
    style RCF fill:#2196F3
```

**Key Design Principles:**
- **Flexible Schema:** `attributes` field in reference classes is a Dict (schema-less)
- **Domain Context:** Git-based JSON prompts have domain-specific terminology
- **Currency/Units:** Stored in `cost_distribution.currency` and `timeline_distribution.unit`
- **Same Algorithm:** Matching and RCF work identically across domains

**Adding a New Domain Checklist:**
1. ✅ Create synthetic data generator in `generators/{domain}.py`
2. ✅ Define domain-specific `attributes` schema
3. ✅ Specify `currency` and `unit` for distributions
4. ✅ Add domain context to prompt config JSON
5. ✅ Add domain to `SUPPORTED_CATEGORIES` list
6. ✅ Seed database with `python seed_database.py --domain {domain}`
7. ✅ Test matching and narrative generation

---

## Technology Stack Summary

```mermaid
graph LR
    subgraph "Frontend"
        W[Widget:<br/>Vite + React 19<br/>TypeScript + Tailwind]
    end
    
    subgraph "Backend"
        API[FastAPI 0.100+<br/>Python 3.11+]
        Motor[Motor 3.0+<br/>Async MongoDB]
    end
    
    subgraph "AI/ML"
        OpenAI[OpenAI GPT-4o-mini<br/>BYOK]
        NumPy[NumPy + SciPy<br/>Synthetic Data]
    end
    
    subgraph "Infrastructure"
        DO[DigitalOcean<br/>App Platform]
        Mongo[(MongoDB Atlas)]
        S3[DO Spaces<br/>S3-Compatible]
        CDN[DO CDN<br/>274 PoPs]
    end
    
    subgraph "DevOps"
        Git[Git-Based Prompts]
        Sentry[Sentry<br/>Monitoring]
        SendGrid[SendGrid<br/>Email]
    end
    
    W --> API
    API --> Motor
    Motor --> Mongo
    API --> OpenAI
    API --> NumPy
    
    API --> DO
    W --> CDN
    CDN --> S3
    
    API --> Git
    API --> Sentry
    API --> SendGrid
    
    style W fill:#61dafb
    style API fill:#009688
    style OpenAI fill:#ff6b6b
    style DO fill:#0080ff
```

---

## Core Architectural Patterns

### 1. Multi-Tenant Isolation (Zero Trust)
```python
# ✅ CORRECT: All queries scoped by tenant
async def get_reference_classes(tenant_id: str, category: str):
    return await db.reference_classes.find({
        "tenant_id": tenant_id,  # REQUIRED
        "category": category
    }).to_list(None)
```

### 2. BYOK Security
- Encrypt OpenAI keys at rest (AES-256)
- Decrypt only at request time
- Never log decrypted keys
- Per-tenant API key isolation

### 3. LLM Retry Logic
- 3 attempts with exponential backoff
- 2-10 second delays
- Graceful degradation on failure
- Sentry error tracking

### 4. Feedback Loop
- Track `prompt_version` with each estimate
- Submit actual outcomes post-project
- Calculate calibration metrics by domain
- Auto-tune distributions over time

### 5. Domain Agnostic
- Flexible `attributes` schema
- Domain-specific prompt contexts
- Currency/unit stored with distributions
- Same matching algorithm for all domains

---

## MVP Scope

**Built in MVP:**
- ✅ Multi-tenant FastAPI backend
- ✅ White label chat widget (React + Shadow DOM)
- ✅ Construction domain (7 subcategories)
- ✅ Synthetic reference class generation
- ✅ Feedback submission system
- ✅ Git-based prompt management
- ✅ BYOK OpenAI integration
- ✅ DO App Platform deployment
- ✅ DO Spaces CDN for widget

**Fast Follow (Post-MVP):**
- ⏳ IT/Dev domain (5 subcategories)
- ⏳ Tenant admin dashboard
- ⏳ Calibration metrics display
- ⏳ "Coach Mode" communication guidance

**Future:**
- 🔮 Agent swarm architecture
- 🔮 Finance domain
- 🔮 Healthcare domain
- 🔮 Tenant-custom domains
- 🔮 Visual drag-and-drop interface
- 🔮 Integration with Jira/Linear/Asana

---

## Key Design Principles

1. **"Trust Through Transparency"** - Make the reasoning visible
2. **"Communication Coach"** - Teach stakeholders probabilistic thinking
3. **Zero Cross-Tenant Data Leakage** - 100% tenant isolation
4. **Self-Improving System** - Feedback loop tunes future estimates
5. **Domain-Agnostic Core** - Same engine, different domains
6. **Distribution Over Distribution** - White label widget beats direct users
7. **MVP Discipline** - Ship fast, iterate with real users

---

_Generated from architecture.md and bmm-brainstorming-session-2025-11-09.md_
_Date: 2025-11-10_
