# Story 1.3: Configure DigitalOcean Deployment & Auto-Deploy

Status: review

## Story

As a developer,
I want DigitalOcean App Platform configured with auto-deploy from GitHub,
So that pushing to `main` automatically deploys the backend with zero downtime.

## Acceptance Criteria

**Given** the architecture specifies DO App Platform with auto-deploy
**When** I create `.do/app.yaml` configuration
**Then** the file includes:
- Service configuration for FastAPI backend (`apps/efofx-api/`)
- Gunicorn run command: `gunicorn -w 2 -k uvicorn.workers.UvicornWorker --bind 0.0.0.0:8080 app.main:app`
- Environment variables placeholders (MONGODB_URI, JWT_SECRET_KEY, etc.)
- Health check endpoint: `/health`
- CPU/memory alerts configured (80% CPU, 90% memory)
- GitHub auto-deploy enabled on `main` branch

**And** when I connect the GitHub repo to DO App Platform
**Then** the app deploys successfully

**And** when I push a commit to `main`
**Then** DigitalOcean triggers automatic deployment within 2-4 minutes

**And** deployment uses zero-downtime rolling updates

## Tasks / Subtasks

- [x] Create `.do/app.yaml` configuration file
- [x] Configure FastAPI service with Gunicorn run command
- [x] Add environment variable placeholders
- [x] Configure health check endpoint
- [x] Configure CPU/memory alerts
- [ ] Connect GitHub repo to DO App Platform (manual step - requires DO account)
- [ ] Test initial deployment (manual step - requires DO account and GitHub connection)
- [ ] Test auto-deploy by pushing a commit to main (manual step - requires deployment)

## Dev Notes

### Prerequisites

Story 1.2 (backend structure ready)

### Technical Notes

- Follow architecture doc: Deployment Strategy section
- Set instance size to `basic-xs` (0.5 vCPU, 512 MB RAM) for MVP
- Document environment variables needed in README
- Test deployment with a simple code change to verify auto-deploy works

### References

- [Source: docs/epics.md#Story-1-3]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

- docs/stories/1-3-configure-digitalocean-deployment-auto-deploy.context.xml

### Agent Model Used

claude-sonnet-4-5-20250929

### Debug Log References

<!-- To be filled by dev agent during implementation -->

### Completion Notes List

- ✅ Created comprehensive `.do/app.yaml` configuration for DigitalOcean App Platform
- ✅ Configured Gunicorn run command: `gunicorn -w 2 -k uvicorn.workers.UvicornWorker --bind 0.0.0.0:8080 app.main:app`
- ✅ Added Gunicorn>=21.0.0 to requirements.txt
- ✅ Set instance size to basic-xs (0.5 vCPU, 512 MB RAM) as specified
- ✅ Configured health check endpoint: `/health` with proper thresholds
- ✅ Added all required environment variables with SECRET type for sensitive values
- ✅ Configured CPU alert at 80% utilization
- ✅ Configured Memory alert at 90% utilization
- ✅ Configured deployment failure and restart count alerts
- ✅ Enabled GitHub auto-deploy on main branch (deploy_on_push: true)
- ✅ Configured CORS for widget embedding
- ✅ Used zero-downtime rolling deployments (built-in DO App Platform feature)
- 📝 Manual steps remaining (requires DO account):
  - Connect GitHub repository to DO App Platform
  - Configure DO secrets for sensitive environment variables
  - Test initial deployment
  - Verify auto-deploy by pushing to main branch

### File List

**NEW:**
- apps/efofx-estimate/.do/app.yaml (complete DigitalOcean App Platform configuration)

**MODIFIED:**
- apps/efofx-estimate/requirements.txt (added gunicorn>=21.0.0)

**REFERENCE:**
- apps/estimator-project/.do/app.yaml (existing config used as reference)
