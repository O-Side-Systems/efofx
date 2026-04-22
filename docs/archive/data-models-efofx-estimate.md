# Data Models - efOfX Estimate (Primary Backend)

**Technology:** Pydantic v2.11.7 (Python data validation)
**Database:** MongoDB (document-based storage)
**ORM:** Motor 3.3.2 (async MongoDB driver)

## Overview

The efOfX Estimation Service uses Pydantic models for data validation, serialization, and API contracts. All models are strongly typed and include validation rules.

## Core Data Models

### Estimation Models

#### `EstimationRequest`

Request model for starting a new estimation.

```python
class EstimationRequest(BaseModel):
    description: str  # min 10, max 2000 characters
    region: Region  # Geographic region enum
    reference_class: Optional[str]  # Optional reference class override
    confidence_threshold: Optional[float]  # 0.0 to 1.0, default 0.7
```

**Example:**
```json
{
  "description": "I want to install a 15x30 foot pool with spa in my backyard.",
  "region": "SoCal - Coastal",
  "reference_class": "residential_pool",
  "confidence_threshold": 0.7
}
```

---

#### `CostBreakdown`

Cost breakdown by category with automatic total calculation.

```python
class CostBreakdown(BaseModel):
    materials: float
    labor: float
    equipment: float  # default 0.0
    permits: float  # default 0.0
    design: float  # default 0.0
    contingency: float  # default 0.0
    profit_margin: float  # default 0.0

    @property
    def total(self) -> float:
        # Auto-calculated sum of all categories
```

**Example:**
```json
{
  "materials": 25000.0,
  "labor": 15000.0,
  "equipment": 5000.0,
  "permits": 2000.0,
  "design": 3000.0,
  "contingency": 5000.0,
  "profit_margin": 8000.0
}
```

**Total:** 63,000.0 (auto-calculated)

---

#### `EstimationResult`

Complete estimation result with confidence metrics.

```python
class EstimationResult(BaseModel):
    total_cost: float
    timeline_weeks: int
    team_size: int
    cost_breakdown: CostBreakdown
    reference_class: str  # Applied reference class
    confidence_score: float  # 0.0 to 1.0
    assumptions: List[str]  # Key assumptions made
    risks: List[str]  # Identified risks
    reference_projects_used: List[str]  # IDs of similar projects
```

**Purpose:** Provides complete estimation with breakdown, confidence, assumptions, and reference data for transparency.

---

### Reference Class Models

#### `ReferenceClass`

Reference class definition for Reference Class Forecasting (RCF).

```python
class ReferenceClass(BaseModel):
    id: Optional[PyObjectId]  # MongoDB ObjectId
    name: str  # e.g., "residential_pool"
    category: ReferenceClassCategory  # Enum: residential, commercial, etc.
    description: str
    keywords: List[str]  # Identifying keywords for matching
    regions: List[Region]  # Applicable regions
    base_cost_per_sqft: Dict[str, float]  # Cost by region
    cost_breakdown_template: Dict[str, float]  # Percentage breakdown
    timeline_multiplier: float  # default 1.0
    team_size_template: Dict[str, int]  # Team size by project size
    tuning_factors: Dict[str, float]  # Region-specific adjustments
    is_active: bool  # default True
    created_at: datetime
    updated_at: datetime
```

**Example:**
```json
{
  "name": "residential_pool",
  "category": "residential",
  "description": "Residential swimming pool installation",
  "keywords": ["pool", "swimming", "backyard", "residential"],
  "regions": ["SoCal - Coastal", "SoCal - Inland"],
  "base_cost_per_sqft": {
    "SoCal - Coastal": 150.0,
    "SoCal - Inland": 140.0
  },
  "cost_breakdown_template": {
    "materials": 0.40,
    "labor": 0.25,
    "equipment": 0.08,
    "permits": 0.03,
    "design": 0.05,
    "contingency": 0.08,
    "profit_margin": 0.11
  },
  "timeline_multiplier": 1.0
}
```

**Purpose:** Defines template for a project type with regional cost variations and breakdown percentages.

---

### Chat Models

#### `ChatRequest`

Request model for chat messages.

```python
class ChatRequest(BaseModel):
    session_id: str  # Estimation session ID
    message: str  # User message
    context: Optional[Dict[str, Any]]  # Additional context
```

---

#### `ChatResponse`

Response model for chat interactions.

```python
class ChatResponse(BaseModel):
    session_id: str
    message: str  # AI response
    suggestions: List[str]  # Follow-up questions
    updated_estimate: Optional[EstimationResult]  # If scope changed
```

---

### Feedback Models

#### `FeedbackCreate`

Request model for submitting feedback.

```python
class FeedbackCreate(BaseModel):
    estimation_id: str
    actual_cost: float
    actual_timeline_weeks: int
    accuracy_rating: float  # 1.0 to 5.0
    comments: Optional[str]
```

---

#### `FeedbackSummary`

Aggregated feedback metrics for a tenant.

```python
class FeedbackSummary(BaseModel):
    total_estimations: int
    average_accuracy: float
    cost_variance: float  # Percentage
    timeline_variance: float  # Percentage
```

---

### Tenant Models

#### `Tenant`

Multi-tenant isolation model.

```python
class Tenant(BaseModel):
    id: PyObjectId
    name: str
    api_key: str  # Hashed
    openai_api_key: Optional[str]  # Encrypted
    region_preferences: List[Region]
    rate_limit_tier: str  # free, pro, enterprise
    is_active: bool
    created_at: datetime
    updated_at: datetime
```

**Purpose:** Isolates data and API keys per tenant for multi-tenant SaaS model.

---

## MongoDB Collections

### Primary Collections

1. **tenants** - Tenant accounts and settings
2. **estimations** - Estimation sessions and results
3. **reference_classes** - Reference class templates
4. **reference_projects** - Historical project data for RCF
5. **chat_sessions** - Chat history per estimation session
6. **feedback** - User-submitted feedback for calibration

### Indexes

- **tenants:** `api_key` (unique)
- **estimations:** `tenant_id`, `session_id` (unique), `status`, `created_at`
- **reference_classes:** `name`, `category`, `is_active`, `regions`
- **reference_projects:** `reference_class`, `region`, `actual_cost`
- **chat_sessions:** `session_id`, `tenant_id`
- **feedback:** `estimation_id`, `tenant_id`, `created_at`

---

## Enums and Constants

### `Region`

```python
class Region(str, Enum):
    SOCAL_COASTAL = "SoCal - Coastal"
    SOCAL_INLAND = "SoCal - Inland"
    NORCAL = "NorCal"
    CENTRAL_COAST = "Central Coast"
    # ... additional regions
```

### `EstimationStatus`

```python
class EstimationStatus(str, Enum):
    PENDING = "pending"
    PROCESSING = "processing"
    COMPLETED = "completed"
    FAILED = "failed"
```

### `ReferenceClassCategory`

```python
class ReferenceClassCategory(str, Enum):
    RESIDENTIAL = "residential"
    COMMERCIAL = "commercial"
    INDUSTRIAL = "industrial"
    INFRASTRUCTURE = "infrastructure"
```

---

## Data Validation Rules

- **Description:** 10-2000 characters
- **Confidence threshold:** 0.0 to 1.0
- **Accuracy rating:** 1.0 to 5.0
- **Cost values:** Must be positive floats
- **Timeline:** Must be positive integers (weeks)

All models use Pydantic's built-in validation with custom error messages for better API responses.
