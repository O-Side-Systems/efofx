# API Contracts - Estimator MCP Functions (Serverless)

**Platform:** DigitalOcean Functions (Serverless)
**Runtime:** Node.js 18+
**Database:** MongoDB 6.3.0
**Deployment:** DigitalOcean App Platform

## Overview

The Estimator MCP Functions provide serverless API endpoints for Reference Class Forecasting data retrieval and manipulation. These lightweight functions complement the main FastAPI backend with high-performance, scalable data access.

## Serverless Functions

### `/manifest`

Returns the manifest of available MCP functions and their metadata.

**Method:** GET
**Authentication:** Optional (public endpoint for discovery)

**Response:**
```json
{
  "functions": [
    {
      "name": "manifest",
      "version": "1.0.0",
      "description": "Returns available functions and metadata"
    },
    {
      "name": "reference_classes-get",
      "version": "1.0.0",
      "description": "Get reference class by ID"
    },
    {
      "name": "reference_classes-query",
      "version": "1.0.0",
      "description": "Query reference classes by criteria"
    },
    {
      "name": "adjustments-apply",
      "version": "1.0.0",
      "description": "Apply regional/complexity adjustments to estimates"
    }
  ]
}
```

---

### `/reference_classes-get`

Get a specific reference class by ID.

**Method:** GET
**Authentication:** Required (JWT token)

**Query Parameters:**
- `id`: Reference class ID (MongoDB ObjectId)

**Response:**
```json
{
  "id": "507f1f77bcf86cd799439011",
  "name": "residential_pool",
  "category": "residential",
  "description": "Residential swimming pool installation",
  "keywords": ["pool", "swimming", "backyard"],
  "regions": ["SoCal - Coastal", "SoCal - Inland"],
  "base_cost_per_sqft": {
    "SoCal - Coastal": 150.0
  },
  "cost_breakdown_template": {...},
  "is_active": true
}
```

**Error Responses:**
- `404 Not Found`: Reference class not found
- `401 Unauthorized`: Missing or invalid JWT

---

### `/reference_classes-query`

Query reference classes by keywords, category, or region.

**Method:** POST
**Authentication:** Required (JWT token)

**Request Body:**
```json
{
  "keywords": ["pool", "backyard"],
  "category": "residential",
  "regions": ["SoCal - Coastal"],
  "limit": 10,
  "offset": 0
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "...",
      "name": "residential_pool",
      "match_score": 0.95,
      "...": "..."
    }
  ],
  "total": 42,
  "limit": 10,
  "offset": 0
}
```

**Purpose:** Find best-matching reference classes for a given project description using keyword matching and similarity scoring.

---

### `/adjustments-apply`

Apply regional and complexity adjustments to baseline estimates.

**Method:** POST
**Authentication:** Required (JWT token)

**Request Body:**
```json
{
  "base_cost": 50000.0,
  "base_timeline_weeks": 8,
  "region": "SoCal - Coastal",
  "complexity_factor": 1.2,
  "risk_factor": 1.1,
  "reference_class": "residential_pool"
}
```

**Response:**
```json
{
  "adjusted_cost": 66000.0,
  "adjusted_timeline_weeks": 10,
  "adjustments_applied": {
    "regional_multiplier": 1.1,
    "complexity_multiplier": 1.2,
    "risk_multiplier": 1.1
  },
  "breakdown": {
    "base": 50000.0,
    "regional": 5000.0,
    "complexity": 10000.0,
    "risk": 1000.0
  }
}
```

**Purpose:** Apply tuning factors to baseline estimates based on regional costs, project complexity, and risk assessments.

---

## Technical Architecture

### Deployment Model

- **Platform:** DigitalOcean Functions (managed serverless)
- **Cold Start:** ~100-300ms
- **Scaling:** Auto-scales based on demand
- **Regions:** Deploy to multiple regions for low latency

### Technology Stack

- **Runtime:** Node.js 18+ (ESM modules)
- **Database:** MongoDB Atlas (shared with FastAPI backend)
- **Validation:** Zod 3.22.4 for request/response schemas
- **Auth:** JWT validation using jsonwebtoken 9.0.2
- **Logging:** Pino 9.0.0 for structured logging
- **Caching:** LRU-cache 10.1.0 for reference class caching

### Performance

- **Average Response Time:** 50-150ms (cached)
- **Cold Start:** ~100-300ms (infrequent)
- **Throughput:** 1000+ req/sec per function (auto-scaled)
- **Cache TTL:** 5 minutes for reference classes

### Monitoring

- DigitalOcean Functions metrics dashboard
- Pino structured logs for debugging
- Error tracking and alerting

---

## Integration with FastAPI Backend

The MCP functions are called by the FastAPI backend for:

1. **Reference Class Lookup** - During estimation initialization
2. **Query Matching** - Finding best reference classes for user descriptions
3. **Adjustment Calculations** - Applying regional/complexity factors

**Call Pattern:**
```
User Request → FastAPI Backend → MCP Functions → MongoDB
                     ↓
              OpenAI API (for LLM)
                     ↓
              Response to User
```

---

## Deployment

Deploy using DigitalOcean CLI:

```bash
doctl serverless deploy .
```

Or with remote build for dependencies:

```bash
doctl serverless deploy . --remote-build
```

Functions are automatically deployed to namespace: `estimator`

---

## Environment Variables

Required environment variables (set in DigitalOcean Functions config):

- `MONGODB_URI`: MongoDB connection string
- `JWT_SECRET`: Secret for JWT validation
- `LOG_LEVEL`: Logging level (info, debug, error)

---

## Error Handling

All functions return standard HTTP status codes:

- `200 OK`: Success
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Missing or invalid JWT
- `404 Not Found`: Resource not found
- `500 Internal Server Error`: Function error

Error response format:
```json
{
  "error": "Error type",
  "message": "Detailed error message",
  "statusCode": 400
}
```
