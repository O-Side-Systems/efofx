# Development Guide - Efofx Platform

**Last Updated:** 2025-11-09
**Target Audience:** Developers setting up local environment or deploying to production

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Local Development Setup](#local-development-setup)
3. [Running the Applications](#running-the-applications)
4. [Testing](#testing)
5. [Deployment](#deployment)
6. [Development Workflows](#development-workflows)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

#### For efofx-estimate (Python Backend)

- **Python:** 3.9 or higher
- **pip:** Latest version
- **MongoDB:** 4.6+ (local or MongoDB Atlas)
- **OpenAI API Key:** For LLM integration

#### For estimator-mcp-functions (Serverless)

- **Node.js:** 18+ (LTS recommended)
- **npm:** Latest version
- **doctl CLI:** DigitalOcean command-line tool
- **DigitalOcean Account:** With API access

### Optional Tools

- **Git:** For version control
- **Docker:** For containerized MongoDB (optional)
- **Postman/Insomnia:** For API testing
- **MongoDB Compass:** For database inspection

---

## Local Development Setup

### 1. Clone the Repository

```bash
git clone <repository-url>
cd efofx-workspace
```

### 2. Set Up efofx-estimate (FastAPI Backend)

#### Navigate to Backend Directory

```bash
cd apps/efofx-estimate
```

#### Create Virtual Environment

```bash
python -m venv .venv

# Activate virtual environment
# macOS/Linux:
source .venv/bin/activate

# Windows:
.venv\Scripts\activate
```

#### Install Dependencies

```bash
pip install -r requirements.txt
```

#### Configure Environment Variables

Create a `.env` file in `apps/efofx-estimate/`:

```env
# MongoDB Configuration
MONGODB_URI=mongodb://localhost:27017/efofx_dev
# Or for MongoDB Atlas:
# MONGODB_URI=mongodb+srv://<user>:<password>@cluster.mongodb.net/efofx_dev

# OpenAI Configuration
OPENAI_API_KEY=sk-...your-key-here...

# JWT Configuration
JWT_SECRET=your-secret-key-here-min-32-chars
JWT_ALGORITHM=HS256

# MCP Functions Configuration
MCP_FUNCTIONS_URL=http://localhost:8080
HMAC_SECRET_B64=<base64-encoded-secret>

# Application Settings
DEBUG=true
LOG_LEVEL=INFO
ENVIRONMENT=development

# CORS Settings
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:8000
```

#### Set Up Local MongoDB (Optional)

If running MongoDB locally instead of Atlas:

```bash
# Using Docker
docker run -d \
  --name mongodb \
  -p 27017:27017 \
  -e MONGO_INITDB_ROOT_USERNAME=admin \
  -e MONGO_INITDB_ROOT_PASSWORD=password \
  mongo:4.6

# Or install MongoDB locally
# macOS:
brew install mongodb-community

# Linux:
sudo apt-get install mongodb
```

#### Seed Database with Reference Data

```bash
python scripts/seed_reference_classes.py
```

### 3. Set Up estimator-mcp-functions (Serverless)

#### Navigate to Functions Directory

```bash
cd apps/estimator-mcp-functions
```

#### Install Dependencies

```bash
npm install
```

#### Configure Environment Variables

Create a `.env` file in `apps/estimator-mcp-functions/`:

```env
# MongoDB Configuration
MONGODB_URI=mongodb://localhost:27017/efofx_dev
DB_NAME=efofx_dev

# Authentication
HMAC_KEY_ID=key-1
HMAC_SECRET_B64=<base64-encoded-secret>
JWT_PUBLIC_KEY_PEM=-----BEGIN PUBLIC KEY-----
...your-public-key...
-----END PUBLIC KEY-----

# Logging
LOG_LEVEL=info
```

#### Install DigitalOcean CLI (doctl)

```bash
# macOS
brew install doctl

# Linux
snap install doctl

# Or download from:
# https://github.com/digitalocean/doctl/releases
```

#### Authenticate with DigitalOcean

```bash
doctl auth init
# Enter your DigitalOcean API token when prompted
```

---

## Running the Applications

### Running efofx-estimate (FastAPI Backend)

#### Development Server with Auto-Reload

```bash
cd apps/efofx-estimate

# Activate virtual environment
source .venv/bin/activate

# Start server
python -m uvicorn app.main:app --reload --host 0.0.0.0 --port 8000
```

The API will be available at:
- **API:** http://localhost:8000
- **Docs (Swagger):** http://localhost:8000/docs
- **ReDoc:** http://localhost:8000/redoc
- **Health Check:** http://localhost:8000/status

#### Production Server

```bash
# Using uvicorn with multiple workers
uvicorn app.main:app \
  --host 0.0.0.0 \
  --port 8000 \
  --workers 4 \
  --log-level info
```

### Running estimator-mcp-functions (Serverless)

#### Deploy to DigitalOcean Functions

```bash
cd apps/estimator-mcp-functions

# Deploy all functions
doctl serverless deploy .

# Deploy with remote build (if using native dependencies)
doctl serverless deploy . --remote-build
```

#### Get Function URLs

```bash
# List all deployed functions
doctl serverless functions list

# Get specific function URL
doctl serverless functions get estimator/manifest --url
doctl serverless functions get estimator/reference_classes-query --url
```

#### Test Functions Locally

```bash
# Test manifest endpoint
doctl serverless functions invoke estimator/manifest

# Test with custom parameters
doctl serverless functions invoke estimator/reference_classes-query \
  --param event='{"tenant_id":"test","attributes":{"category":"construction"}}'
```

#### View Function Logs

```bash
# Stream logs in real-time
doctl serverless activations logs --follow

# View logs for specific function
doctl serverless activations logs --function estimator/reference_classes-query

# View recent activations
doctl serverless activations list
```

---

## Testing

### Testing efofx-estimate

#### Run All Tests

```bash
cd apps/efofx-estimate

# Activate virtual environment
source .venv/bin/activate

# Run all tests
pytest

# Run with coverage
pytest --cov=app --cov-report=html

# View coverage report
open htmlcov/index.html
```

#### Run Specific Test Categories

```bash
# Unit tests only
pytest tests/unit/

# Integration tests only
pytest tests/integration/

# API endpoint tests
pytest tests/api/

# Service layer tests
pytest tests/services/

# Specific test file
pytest tests/api/test_estimation.py

# Specific test function
pytest tests/api/test_estimation.py::test_start_estimation
```

#### Run Tests with Verbose Output

```bash
# Verbose mode
pytest -v

# Show print statements
pytest -s

# Stop on first failure
pytest -x
```

### Testing estimator-mcp-functions

#### Run All Tests

```bash
cd apps/estimator-mcp-functions

npm test
```

#### Run Tests with Coverage

```bash
npm test -- --coverage
```

#### Linting and Formatting

```bash
# Lint code
npm run lint

# Check formatting
npm run format:check

# Auto-fix formatting
npm run format
```

---

## Deployment

### Deploying efofx-estimate

#### DigitalOcean App Platform

1. **Create App:**
   - Go to DigitalOcean App Platform console
   - Click "Create App"
   - Connect to GitHub repository

2. **Configure Build:**
   ```yaml
   # .do/app.yaml
   name: efofx-estimate
   services:
     - name: api
       build_command: pip install -r requirements.txt
       run_command: uvicorn app.main:app --host 0.0.0.0 --port 8080
       environment_slug: python
       instance_count: 2
       instance_size_slug: basic-xs
       envs:
         - key: MONGODB_URI
           scope: RUN_TIME
           type: SECRET
         - key: OPENAI_API_KEY
           scope: RUN_TIME
           type: SECRET
         - key: JWT_SECRET
           scope: RUN_TIME
           type: SECRET
   ```

3. **Set Environment Variables:**
   - Navigate to Settings > App-Level Environment Variables
   - Add all required variables from `.env` template

4. **Deploy:**
   - Click "Create Resources"
   - Monitor build logs
   - Access app at provided URL

#### DigitalOcean Droplet (Manual)

```bash
# SSH into droplet
ssh root@your-droplet-ip

# Install Python
apt-get update
apt-get install -y python3.9 python3-pip

# Clone repository
git clone <repository-url>
cd efofx-workspace/apps/efofx-estimate

# Install dependencies
pip3 install -r requirements.txt

# Create systemd service
cat > /etc/systemd/system/efofx-estimate.service <<EOF
[Unit]
Description=Efofx Estimation Service
After=network.target

[Service]
User=www-data
WorkingDirectory=/path/to/efofx-estimate
Environment="PATH=/usr/bin"
EnvironmentFile=/path/to/.env
ExecStart=/usr/bin/uvicorn app.main:app --host 0.0.0.0 --port 8000 --workers 4

[Install]
WantedBy=multi-user.target
EOF

# Start service
systemctl enable efofx-estimate
systemctl start efofx-estimate
```

### Deploying estimator-mcp-functions

#### Deploy to Functions Namespace

```bash
cd apps/estimator-mcp-functions

# Deploy all functions
doctl serverless deploy .

# Verify deployment
doctl serverless functions list

# Check function health
curl https://<function-url>/estimator/manifest
```

#### Deploy to App Platform

1. **Create App Component:**
   - Add "Serverless Functions" component
   - Point to repository path: `apps/estimator-mcp-functions`

2. **Configure Environment Variables:**
   - Set `MONGODB_URI`, `HMAC_SECRET_B64`, etc.

3. **Deploy:**
   - App Platform will auto-deploy on git push

#### Rollback Deployment

```bash
# List deployments
doctl serverless deployments list

# Rollback to specific deployment
doctl serverless deployments rollback <deployment-id>
```

---

## Development Workflows

### Adding a New API Endpoint (FastAPI)

1. **Define Pydantic Models:**
   ```python
   # app/models/my_feature.py
   from pydantic import BaseModel

   class MyRequest(BaseModel):
       field1: str
       field2: int

   class MyResponse(BaseModel):
       result: str
   ```

2. **Create Service Layer:**
   ```python
   # app/services/my_feature_service.py
   async def my_business_logic(request: MyRequest) -> MyResponse:
       # Implement logic here
       return MyResponse(result="success")
   ```

3. **Add Route:**
   ```python
   # app/api/routes.py
   from app.models.my_feature import MyRequest, MyResponse
   from app.services.my_feature_service import my_business_logic

   @api_router.post("/my-endpoint", response_model=MyResponse)
   async def my_endpoint(
       request: MyRequest,
       tenant: Tenant = Depends(get_current_tenant)
   ):
       return await my_business_logic(request)
   ```

4. **Write Tests:**
   ```python
   # tests/api/test_my_feature.py
   def test_my_endpoint(client, auth_headers):
       response = client.post(
           "/my-endpoint",
           json={"field1": "test", "field2": 123},
           headers=auth_headers
       )
       assert response.status_code == 200
   ```

### Adding a New Serverless Function

1. **Create Function Directory:**
   ```bash
   mkdir -p packages/estimator/my-function
   cd packages/estimator/my-function
   ```

2. **Create Handler:**
   ```javascript
   // index.js
   import { db } from '../../../lib/db.js';
   import { verifyHmac } from '../../../lib/auth.js';

   export async function main(event) {
     const auth = verifyHmac({ event, secretBase64: process.env.HMAC_SECRET_B64 });
     if (!auth.ok) return { statusCode: 401, body: { error: 'unauthorized' } };

     // Your logic here
     const result = await (await db()).collection('my_collection').findOne({});

     return { body: result };
   }
   ```

3. **Add package.json:**
   ```json
   {
     "name": "my-function",
     "version": "1.0.0",
     "main": "index.js"
   }
   ```

4. **Update project.yml:**
   ```yaml
   functions:
     - name: my-function
       runtime: "nodejs:18"
       web: true
       limits: { timeout: 2000, memory: 256 }
   ```

5. **Deploy and Test:**
   ```bash
   doctl serverless deploy .
   doctl serverless functions invoke estimator/my-function
   ```

### Running Database Migrations

```bash
cd apps/efofx-estimate

# Run migration script
python scripts/migrate_db.py

# Or create new migration
python scripts/migrations/create_migration.py add_new_field
```

### Analyzing Feedback Data

```bash
cd apps/efofx-estimate

# Generate feedback summary
python scripts/analyze_feedback.py --tenant-id acme-co

# Export calibration metrics
python scripts/analyze_feedback.py --export-csv feedback_report.csv
```

---

## Troubleshooting

### Common Issues

#### FastAPI Issues

**Issue:** `ModuleNotFoundError: No module named 'app'`
**Solution:**
```bash
# Make sure you're in the correct directory
cd apps/efofx-estimate

# Run with python -m
python -m uvicorn app.main:app --reload
```

**Issue:** `MongoDB connection timeout`
**Solution:**
```bash
# Check MongoDB is running
docker ps | grep mongodb

# Verify connection string in .env
echo $MONGODB_URI

# Test connection
python -c "from pymongo import MongoClient; client = MongoClient('mongodb://localhost:27017'); print(client.server_info())"
```

**Issue:** `JWT validation failed`
**Solution:**
- Verify `JWT_SECRET` matches between token generation and validation
- Check token expiry time
- Ensure tenant_id claim is present

#### MCP Functions Issues

**Issue:** `Cold start delays`
**Solution:**
- Expected on first request (100-300ms)
- Subsequent requests use warm containers
- Consider increasing memory limits in project.yml for faster cold starts

**Issue:** `HMAC signature verification failed`
**Solution:**
```bash
# Verify HMAC_SECRET_B64 matches between caller and function
echo $HMAC_SECRET_B64 | base64 -d

# Check timestamp skew (must be < 120 seconds)
date +%s

# Generate new HMAC signature
python scripts/generate_hmac.py
```

**Issue:** `MongoDB connection pool exhausted`
**Solution:**
- Increase `maxPoolSize` in `lib/db.js`
- Monitor connection usage in MongoDB Atlas
- Ensure connections are properly closed

#### Testing Issues

**Issue:** `Tests failing with database errors`
**Solution:**
```bash
# Use separate test database
export MONGODB_URI=mongodb://localhost:27017/efofx_test

# Clear test database between runs
python scripts/clear_test_db.py
```

**Issue:** `OpenAI API rate limit errors in tests`
**Solution:**
- Mock OpenAI API calls in tests (see `tests/fixtures/`)
- Use test API key with higher rate limits
- Add retry logic with exponential backoff

### Performance Optimization

#### FastAPI Optimization

- **Connection Pooling:** Increase MongoDB pool size for high traffic
  ```python
  # app/db/mongodb.py
  client = AsyncIOMotorClient(
      settings.MONGODB_URI,
      maxPoolSize=50,  # Increase from default 10
      minPoolSize=10
  )
  ```

- **Response Caching:** Cache frequently accessed data
  ```python
  from functools import lru_cache

  @lru_cache(maxsize=100)
  async def get_reference_class(rc_id: str):
      # Cached for 5 minutes
      pass
  ```

#### MCP Functions Optimization

- **Increase Memory:** Faster execution
  ```yaml
  # project.yml
  limits: { timeout: 2000, memory: 512 }  # Double memory
  ```

- **Connection Reuse:** Module-scope MongoDB client already implemented in `lib/db.js`

- **LRU Cache:** Already implemented in `lib/cache.js` (5min TTL)

### Debugging

#### Enable Debug Logging (FastAPI)

```bash
# In .env
LOG_LEVEL=DEBUG

# Or at runtime
LOG_LEVEL=DEBUG python -m uvicorn app.main:app --reload
```

#### Enable Debug Logging (MCP Functions)

```bash
# In .env
LOG_LEVEL=debug

# View function logs
doctl serverless activations logs --follow
```

#### Database Query Profiling

```bash
# MongoDB Atlas
# Enable profiling in Atlas UI: Database > Performance Advisor

# Local MongoDB
mongo
use efofx_dev
db.setProfilingLevel(2)  # Profile all queries
db.system.profile.find().pretty()
```

---

## Code Style and Standards

### Python (FastAPI)

- **Style Guide:** [Google Python Style Guide](https://google.github.io/styleguide/pyguide.html)
- **Formatter:** black
- **Linter:** flake8
- **Type Checking:** mypy

```bash
# Format code
black app/ tests/

# Lint
flake8 app/ tests/

# Type check
mypy app/
```

### JavaScript (MCP Functions)

- **Linter:** ESLint
- **Formatter:** Prettier

```bash
# Format code
npm run format

# Lint
npm run lint

# Fix auto-fixable issues
npm run lint -- --fix
```

### Docstring Style (Python)

```python
def my_function(param1: str, param2: int) -> dict:
    """Brief description of function.

    Longer description if needed.

    Args:
        param1: Description of param1
        param2: Description of param2

    Returns:
        Description of return value

    Raises:
        ValueError: When validation fails
    """
    pass
```

---

## Useful Commands Reference

### efofx-estimate (FastAPI)

```bash
# Start development server
python -m uvicorn app.main:app --reload

# Run tests
pytest

# Run tests with coverage
pytest --cov=app

# Format code
black app/ tests/

# Lint code
flake8 app/ tests/

# Type check
mypy app/

# Seed database
python scripts/seed_reference_classes.py
```

### estimator-mcp-functions

```bash
# Deploy functions
doctl serverless deploy .

# List functions
doctl serverless functions list

# Get function URL
doctl serverless functions get estimator/manifest --url

# Invoke function
doctl serverless functions invoke estimator/manifest

# View logs
doctl serverless activations logs --follow

# Run tests
npm test

# Lint
npm run lint

# Format
npm run format
```

---

_Generated by BMad Method document-project workflow (Deep Scan)_
_Last Updated: 2025-11-09_
