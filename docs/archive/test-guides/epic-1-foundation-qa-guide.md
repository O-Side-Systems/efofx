# Epic 1: Foundation & Infrastructure - Manual QA Test Guide

**Version:** 1.1
**Date:** 2025-11-16 (Sentry integration removed)
**Epic:** Foundation & Infrastructure Setup
**Tester Profile:** No prior codebase knowledge required

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Environment Setup](#environment-setup)
3. [Test Cases](#test-cases)
4. [Expected Results](#expected-results)
5. [Known Limitations](#known-limitations)
6. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Accounts

Before starting, ensure you have access to:

- ✅ **GitHub Account** - For repository access and auto-deploy testing
- ✅ **MongoDB Atlas Account** - Free tier (M0) is sufficient
  - Sign up at: https://www.mongodb.com/cloud/atlas
- ✅ **DigitalOcean Account** - For deployment testing (optional for basic testing)
  - Sign up at: https://www.digitalocean.com/

**Note:** Sentry integration has been removed. Error tracking uses structured logging via `structlog` instead.

### Required Tools

Install the following tools on your machine:

- ✅ **Node.js 20+** - JavaScript runtime
  - Download: https://nodejs.org/
  - Verify: `node --version` (should be 20.0.0 or higher)

- ✅ **Python 3.11+** - Programming language
  - Download: https://www.python.org/downloads/
  - Verify: `python3 --version` (should be 3.11.0 or higher)

- ✅ **npm** - Node package manager (comes with Node.js)
  - Verify: `npm --version`

- ✅ **git** - Version control
  - Download: https://git-scm.com/
  - Verify: `git --version`

- ✅ **curl** - Command line tool for testing HTTP endpoints
  - Usually pre-installed on macOS/Linux
  - Windows: Download from https://curl.se/
  - Verify: `curl --version`

### Test Data Requirements

**None** - Epic 1 is infrastructure setup. No test data is required.

---

## Environment Setup

### Step 1: Clone the Repository

```bash
# Clone the repository
git clone https://github.com/[your-org]/efofx-workspace.git

# Navigate to the project directory
cd efofx-workspace

# Verify you're in the correct directory
pwd
# Expected output: /path/to/efofx-workspace
```

### Step 2: Setup Widget (Frontend)

```bash
# Navigate to widget directory
cd apps/efofx-widget

# Install dependencies
npm install

# Expected output:
# - "added X packages" message
# - No error messages
# - Takes ~10-30 seconds

# Build the widget
npm run build

# Expected output:
# - "dist/embed.js" file created
# - Build time ~1-2 seconds
# - No errors

# Verify build output
ls -lh dist/

# Expected output:
# - embed.js file (~578KB)
# - vite.svg file
```

### Step 3: Setup Backend (API)

```bash
# Navigate to backend directory
cd ../../apps/efofx-estimate

# Create virtual environment
python3 -m venv .venv

# Activate virtual environment
source .venv/bin/activate  # macOS/Linux
# .venv\Scripts\activate  # Windows

# Your terminal prompt should now show (.venv)

# Install dependencies
pip install -r requirements.txt

# Expected output:
# - "Successfully installed..." messages
# - Takes ~1-2 minutes
# - No error messages
```

### Step 4: Configure Environment Variables

```bash
# Still in apps/efofx-estimate directory

# Copy example environment file
cp .env.example .env

# Edit .env file (use your preferred text editor)
nano .env  # or: code .env, vim .env, etc.

# REQUIRED: Update these values:
# MONGO_URI=mongodb+srv://[username]:[password]@[cluster].mongodb.net/efofx_production
# OPENAI_API_KEY=sk-[your-openai-api-key]
# SECRET_KEY=[generate with: python -c "import secrets; print(secrets.token_urlsafe(32))"]
# JWT_SECRET_KEY=[generate with: python -c "import secrets; print(secrets.token_urlsafe(32))"]
# ENCRYPTION_KEY=[generate with: python -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())"]

# Save and close the file
```

**MongoDB Atlas Setup:**
1. Go to https://www.mongodb.com/cloud/atlas
2. Create a free M0 cluster
3. Click "Connect" → "Connect your application"
4. Copy the connection string and paste it into MONGO_URI in .env
5. Replace `<password>` with your database password
6. Change database name from `test` to `efofx_production`

---

## Test Cases

### TC1.1: Widget Builds Successfully

**Objective:** Verify the widget project builds and produces a single embed.js bundle

**Steps:**
```bash
cd apps/efofx-widget
npm run build
ls -lh dist/
```

**Expected Results:**
- ✅ Build completes without errors
- ✅ `dist/embed.js` file exists
- ✅ File size is < 600KB (actual: ~578KB)
- ✅ Build time is < 5 seconds

**Pass Criteria:** All checkboxes above are ✅

---

### TC1.2: Backend Starts Successfully

**Objective:** Verify the FastAPI backend starts without errors

**Steps:**
```bash
cd apps/efofx-estimate
source .venv/bin/activate
uvicorn app.main:app --reload
```

**Expected Output:**
```
INFO:     Uvicorn running on http://127.0.0.1:8000 (Press CTRL+C to quit)
INFO:     Started reloader process
INFO:     Started server process
INFO:     Waiting for application startup.
INFO:     Starting efOfX Estimation Service...
INFO:     MongoDB connection established
INFO:     Application startup complete.
```

**Expected Results:**
- ✅ Server starts on port 8000
- ✅ No error messages in console
- ✅ MongoDB connection message appears
- ✅ Application startup complete message appears

**Pass Criteria:** All checkboxes above are ✅

**Keep the server running for the next tests.**

---

### TC1.3: Health Endpoint Returns 200 OK

**Objective:** Verify the /health endpoint is accessible and returns correct status

**Steps:**
```bash
# In a NEW terminal window (keep server running in first terminal)
curl http://localhost:8000/health
```

**Expected Output:**
```json
{
  "status": "healthy",
  "service": "efOfX Estimation Service",
  "database": "connected",
  "version": "1.0.0"
}
```

**Expected Results:**
- ✅ HTTP 200 status code
- ✅ Response time < 100ms
- ✅ `status` field is "healthy"
- ✅ `database` field is "connected"

**Pass Criteria:** All checkboxes above are ✅

---

### TC1.4: MongoDB Connection Status in Health Response

**Objective:** Verify health endpoint includes database status check

**Steps:**
```bash
curl http://localhost:8000/health | jq '.database'
```

**Expected Output:**
```
"connected"
```

**Expected Results:**
- ✅ Database field is "connected"
- ✅ If MongoDB is unavailable, field changes to "disconnected" and status becomes "degraded"

**Pass Criteria:** Database status correctly reflects MongoDB connection state

---

### TC1.5: Error Logging Works (Sentry Removed)

**Objective:** Verify structured logging captures errors correctly

**Status:** ⚠️ **UPDATED** - Sentry integration removed, now using `structlog` for error tracking

**Steps:**
```bash
# Check that server logs errors correctly
# You can trigger an error by requesting a non-existent endpoint
curl http://localhost:8000/nonexistent-endpoint
```

**Expected Output:**
```json
{
  "detail": "Not Found"
}
```

**Verification Steps:**
1. Check server logs - should see structured error logging
2. Logs should be in JSON format with timestamp, level, and message
3. All errors are logged to stdout (captured by DigitalOcean App Platform in production)

**Expected Results:**
- ✅ Error is logged to console with structured format
- ✅ No application crash
- ✅ HTTP 404 response returned

**Pass Criteria:** Errors are logged in structured format

**Note:** Sentry has been removed. Error tracking now relies on structured logging via `structlog`. For production error monitoring, logs are available in DigitalOcean App Platform dashboard.

---

### TC1.6: DigitalOcean Deployment Configuration Exists

**Objective:** Verify .do/app.yaml configuration file is created and valid

**Steps:**
```bash
cd apps/efofx-estimate
cat .do/app.yaml
```

**Expected Results:**
- ✅ File exists at `.do/app.yaml`
- ✅ Contains Gunicorn run command
- ✅ Health check endpoint configured as `/health`
- ✅ CPU alert configured at 80%
- ✅ Memory alert configured at 90%
- ✅ GitHub auto-deploy enabled (`deploy_on_push: true`)
- ✅ Instance size is `basic-xs`

**Pass Criteria:** File exists with all required configuration

**Note:** Actual deployment testing requires DigitalOcean account and is beyond scope of this guide.

---

### TC1.7: Widget Embeds in Test HTML Page

**Objective:** Verify widget can be embedded in a test HTML page

**Steps:**
```bash
cd apps/efofx-widget

# Open test-embed.html in browser
# macOS:
open test-embed.html

# Linux:
xdg-open test-embed.html

# Windows:
start test-embed.html
```

**Expected Results:**
- ✅ Page loads without console errors
- ✅ Widget container is visible
- ✅ Widget initialization message in console: "efOfX Widget initialized successfully!"
- ✅ No CSS conflicts with page styles (widget is isolated via Shadow DOM)
- ✅ React dev tools show widget component tree

**Visual Verification:**
- Page shows header "efOfX Widget Test Page"
- Widget loads in the designated container
- Widget styles don't affect page header styles

**Pass Criteria:** Widget loads and renders without errors

---

## Expected Results

### Summary

When all test cases pass, you should have:

| Component | Expected State |
|-----------|---------------|
| **Widget Build** | Single `embed.js` file (~578KB) |
| **Backend Server** | Running on port 8000, no errors |
| **Health Endpoint** | Returns 200 OK, response time < 100ms |
| **MongoDB** | Connected, status visible in /health |
| **Error Logging** | Structured logs via structlog |
| **DO Config** | Complete app.yaml with alerts |
| **Widget Embed** | Loads in test HTML page |

### Performance Benchmarks

- Widget bundle size: < 600KB ✅ (actual: 578KB)
- Backend health check: < 100ms ✅
- MongoDB connection: Established on startup ✅
- Error logging: Real-time to stdout ✅

---

## Known Limitations

At this stage (Epic 1 complete), the following are expected limitations:

### API Endpoints
- ❌ **No business logic endpoints** - Only `/health`, `/test-error`, and `/` exist
- ❌ **No estimation API** - Will be implemented in Epic 2
- ❌ **No tenant management** - Will be implemented in Epic 3
- ❌ **No authentication** - Will be implemented in Epic 3

### Database
- ❌ **No collections created** - Database exists but is empty
- ❌ **No indexes defined** - Will be created when collections are added
- ❌ **No seed data** - Synthetic data generation is Epic 2

### Widget
- ❌ **Placeholder content only** - Widget renders but has no chat functionality
- ❌ **No API integration** - Will connect to backend in Epic 4
- ❌ **No branding system** - Will be implemented in Epic 5

### Deployment
- ❌ **Not deployed to DigitalOcean** - Manual deployment required
- ❌ **No production environment** - Configuration exists, actual deployment is manual
- ❌ **Auto-deploy not tested** - Requires GitHub repo connection to DO

These limitations are **expected and acceptable** for Epic 1. They will be addressed in subsequent epics.

---

## Troubleshooting

### Issue: Widget build fails with "tailwindcss" error

**Symptoms:**
```
[postcss] It looks like you're trying to use `tailwindcss` directly
```

**Solution:**
```bash
npm install @tailwindcss/postcss
# Verify postcss.config.js uses '@tailwindcss/postcss'
```

---

### Issue: Backend fails to start - "ImportError: No module named"

**Symptoms:**
```
ImportError: No module named 'fastapi'
```

**Solution:**
```bash
# Ensure virtual environment is activated
source .venv/bin/activate  # Your prompt should show (.venv)

# Reinstall dependencies
pip install -r requirements.txt
```

---

### Issue: MongoDB connection fails

**Symptoms:**
```
ERROR: Failed to connect to MongoDB
```

**Solution:**
1. Verify MONGO_URI in .env is correct
2. Check MongoDB Atlas:
   - Cluster is running (not paused)
   - IP whitelist includes your IP (or use 0.0.0.0/0 for testing)
   - Database user credentials are correct
3. Test connection:
   ```bash
   # Use mongosh or MongoDB Compass to verify connection string
   ```

---

### Issue: Health endpoint returns "database": "disconnected"

**Symptoms:**
```json
{"status": "degraded", "database": "disconnected"}
```

**Solution:**
1. Check server logs for MongoDB connection error
2. Verify MONGO_URI in .env
3. Restart server after fixing .env:
   ```bash
   # Press CTRL+C to stop
   uvicorn app.main:app --reload
   ```

---

### Issue: Errors not being logged

**Symptoms:**
- Errors occur but no logs appear in console

**Solution:**
1. Verify logging is configured correctly in app/main.py
2. Check that `structlog` is installed: `pip list | grep structlog`
3. Ensure console output is not being suppressed
4. Verify DEBUG mode is enabled in .env for detailed logs
5. Check that you're looking at the correct terminal window (where uvicorn is running)

**Note:** Sentry integration has been removed. All errors are logged to stdout using `structlog`.

---

### Issue: Port 8000 already in use

**Symptoms:**
```
ERROR: [Errno 48] Address already in use
```

**Solution:**
```bash
# Find process using port 8000
lsof -i :8000

# Kill the process
kill -9 [PID]

# Or use a different port
uvicorn app.main:app --reload --port 8001
```

---

## Approval Checklist

Before marking Epic 1 as complete, verify:

- [ ] **TC1.1** - Widget builds successfully (embed.js < 600KB)
- [ ] **TC1.2** - Backend starts without errors
- [ ] **TC1.3** - Health endpoint returns 200 OK
- [ ] **TC1.4** - MongoDB connection status is "connected"
- [ ] **TC1.5** - Error logging works via structlog
- [ ] **TC1.6** - DigitalOcean app.yaml exists with correct config
- [ ] **TC1.7** - Widget embeds in test HTML page

**Epic 1 is approved when all checkboxes above are ✅**

---

## Next Steps

After Epic 1 approval:

1. **Epic 2**: Reference Class Engine & Synthetic Data
   - Implement MongoDB schemas
   - Create RCF matching algorithm
   - Generate synthetic construction data
   - Implement baseline estimation calculation

2. **Epic 3**: Multi-Tenant Infrastructure & Security
   - Tenant registration and management
   - JWT authentication
   - BYOK encryption for API keys
   - Tenant isolation middleware

3. **Epic 4**: LLM Integration & Conversational Scoping
   - OpenAI client with BYOK support
   - Git-based prompt management
   - Estimate narratives generation
   - Conversational chat engine

---

**Document Version:** 1.1
**Last Updated:** 2025-11-16 (Sentry integration removed)
**Next Review:** After Epic 1 QA completion
