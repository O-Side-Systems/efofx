# efOfX Estimation Service - Setup Guide

## Prerequisites

- Python 3.11+ installed
- MongoDB (local or Atlas connection)
- OpenAI API key

## Environment Setup

### 1. Create Virtual Environment

```bash
# Navigate to backend directory
cd apps/efofx-estimate

# Remove old virtual environment if exists
rm -rf .venv

# Create new virtual environment
python3 -m venv .venv

# Activate virtual environment
source .venv/bin/activate  # On macOS/Linux
# .venv\Scripts\activate  # On Windows
```

### 2. Install Dependencies

```bash
# Install all dependencies from requirements.txt
pip install -r requirements.txt
```

### 3. Configure Environment Variables

```bash
# Copy example environment file
cp .env.example .env

# Edit .env and fill in your actual values:
# - MONGO_URI (MongoDB connection string)
# - OPENAI_API_KEY (from OpenAI platform)
# - SECRET_KEY (generate with: python -c "import secrets; print(secrets.token_urlsafe(32))")
# - JWT_SECRET_KEY (generate with same command)
# - ENCRYPTION_KEY (generate with: python -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())")
# - SENTRY_DSN (from Sentry dashboard, if using)
```

### 4. Start Development Server

```bash
# Run with uvicorn (auto-reload enabled)
uvicorn app.main:app --reload

# Server will start at http://localhost:8000
# API docs available at http://localhost:8000/docs
```

### 5. Verify Installation

```bash
# Test health endpoint
curl http://localhost:8000/health

# Expected response:
# {"status": "healthy", "service": "efOfX Estimation Service"}
```

## Project Structure

```
apps/efofx-estimate/
├── app/
│   ├── api/           # API routes and endpoints
│   ├── core/          # Config, security, logging
│   ├── db/            # MongoDB connection
│   ├── middleware/    # Tenant isolation, CORS, rate limiting
│   ├── models/        # Pydantic models
│   ├── services/      # Business logic
│   └── utils/         # Shared utilities
├── config/
│   └── prompts/       # Git-based LLM prompts (JSON)
├── tests/             # Test suite
├── .env               # Environment variables (not in git)
├── .env.example       # Example environment file
└── requirements.txt   # Python dependencies
```

## Next Steps

1. Run test suite: `pytest`
2. Review API documentation at `/docs`
3. Implement first API endpoints (see Epic 2 stories)
