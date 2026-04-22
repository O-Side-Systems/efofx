# SaaS B2B Backend Specific Requirements

## Multi-Tenancy Architecture

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

## Authentication & Authorization

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

## Data Schemas

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

## API Endpoints (Complete Specification)

### Tenant Management API

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

### Estimation API

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

### Chat API (Conversational Scoping)

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

### Feedback API

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

### Widget API

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

### Synthetic Data API (Admin/Internal)

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
