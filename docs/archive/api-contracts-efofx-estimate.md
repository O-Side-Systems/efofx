# API Contracts - efOfX Estimate (Primary Backend)

**Service:** efOfX Estimation Service
**Version:** 1.0.0
**Base URL:** `/api/v1`
**Authentication:** JWT Bearer Token (tenant-based multi-tenancy)

## Overview

The efOfX Estimation Service provides a RESTful API for AI-powered project estimation using Reference Class Forecasting (RCF). All endpoints require tenant authentication via JWT tokens.

## Authentication

All endpoints (except `/status`) require a valid JWT token in the Authorization header:

```
Authorization: Bearer <jwt_token>
```

Rate limiting is applied per tenant to prevent abuse.

## API Endpoints

### Estimation Endpoints

#### `POST /estimate/start`

Start a new estimation session.

**Request Body:**
```json
{
  "description": "I want to install a 15x30 foot pool with spa in my backyard.",
  "region": "SoCal - Coastal",
  "reference_class": "residential_pool",
  "confidence_threshold": 0.7
}
```

**Response:** `EstimationResponse`
- `session_id`: Unique session identifier
- `status`: Estimation status (pending, processing, completed, failed)
- `result`: Estimation result (when completed)

**Authentication:** Required (tenant JWT)
**Rate Limited:** Yes

---

#### `GET /estimate/{session_id}`

Get estimation session status and results.

**Path Parameters:**
- `session_id`: Unique session identifier

**Response:** `EstimationResponse`

**Authentication:** Required (tenant JWT)
**Rate Limited:** No

---

#### `POST /estimate/{session_id}/upload`

Upload image for estimation session (e.g., site photos, floor plans).

**Path Parameters:**
- `session_id`: Unique session identifier

**Request:** Multipart form data with image file

**Response:**
```json
{
  "message": "Image uploaded successfully",
  "image_url": "https://storage.example.com/uploads/..."
}
```

**Authentication:** Required (tenant JWT)
**Rate Limited:** Yes

---

### Chat Endpoints

#### `POST /chat/send`

Send a chat message for estimation (conversational scoping).

**Request Body:**
```json
{
  "session_id": "uuid",
  "message": "I also want a built-in spa",
  "context": {}
}
```

**Response:** `ChatResponse`
- `session_id`: Session identifier
- `message`: AI response
- `suggestions`: Follow-up questions or clarifications
- `updated_estimate`: Updated estimation if scope changed

**Authentication:** Required (tenant JWT)
**Rate Limited:** Yes

---

#### `GET /chat/{session_id}/history`

Get chat history for a session.

**Path Parameters:**
- `session_id`: Unique session identifier

**Response:**
```json
{
  "session_id": "uuid",
  "messages": [
    {
      "role": "user",
      "content": "...",
      "timestamp": "2025-11-09T10:00:00Z"
    },
    {
      "role": "assistant",
      "content": "...",
      "timestamp": "2025-11-09T10:00:05Z"
    }
  ]
}
```

**Authentication:** Required (tenant JWT)
**Rate Limited:** No

---

### Feedback Endpoints

#### `POST /feedback/submit`

Submit feedback for an estimation (actual vs estimated outcomes).

**Request Body:**
```json
{
  "estimation_id": "uuid",
  "actual_cost": 55000.0,
  "actual_timeline_weeks": 10,
  "accuracy_rating": 4.5,
  "comments": "Estimate was close, timeline was accurate"
}
```

**Response:**
```json
{
  "message": "Feedback submitted successfully",
  "feedback_id": "uuid"
}
```

**Authentication:** Required (tenant JWT)
**Rate Limited:** Yes

---

#### `GET /feedback/summary`

Get feedback summary for tenant (calibration metrics).

**Response:** `FeedbackSummary`
- `total_estimations`: Total estimations performed
- `average_accuracy`: Average accuracy rating
- `cost_variance`: Average cost variance percentage
- `timeline_variance`: Average timeline variance percentage

**Authentication:** Required (tenant JWT)
**Rate Limited:** No

---

### Health & Status Endpoints

#### `GET /status`

Get service status and health information.

**Response:**
```json
{
  "status": "healthy",
  "database": {
    "connected": true,
    "stats": {
      "collections": 5,
      "documents": 1234
    }
  },
  "service": "efOfX Estimation Service",
  "version": "1.0.0"
}
```

**Authentication:** Not required
**Rate Limited:** No

---

### Admin Endpoints

#### `GET /admin/tenants`

List tenants (admin only).

**Query Parameters:**
- `limit`: Number of tenants to return (default: 10)
- `offset`: Offset for pagination (default: 0)

**Response:**
```json
{
  "tenants": [...],
  "total": 42
}
```

**Authentication:** Required (admin tenant JWT)
**Rate Limited:** No

---

## Error Responses

All endpoints return standard HTTP status codes:

- `200 OK`: Success
- `400 Bad Request`: Invalid request data
- `401 Unauthorized`: Missing or invalid JWT token
- `404 Not Found`: Resource not found
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error

Error response format:
```json
{
  "detail": "Error message description"
}
```

## Rate Limiting

- **Estimation endpoints:** 10 requests per minute per tenant
- **Chat endpoints:** 30 requests per minute per tenant
- **Feedback endpoints:** 5 requests per minute per tenant
- **Other endpoints:** No rate limit

Rate limit headers are included in responses:
- `X-RateLimit-Limit`: Maximum requests allowed
- `X-RateLimit-Remaining`: Remaining requests
- `X-RateLimit-Reset`: Time when limit resets (Unix timestamp)
