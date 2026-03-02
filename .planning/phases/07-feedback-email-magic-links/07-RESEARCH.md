# Phase 7: Feedback Email & Magic Links - Research

**Researched:** 2026-03-01
**Domain:** Transactional email (Resend SDK), magic link token security, FastAPI background tasks, MongoDB TTL indexes, React feedback form UI
**Confidence:** HIGH (architecture well-understood from codebase + official Resend docs verified)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Email Trigger & Sending**
- Manual "Request Feedback" button — contractor clicks on a delivered estimate to trigger the email
- No automatic sending — contractor controls timing and which customers to ask
- Single efOfX sending domain (e.g., feedback@efofx.com) — not per-tenant sender addresses
- Resend as transactional email provider via official Python SDK (`resend` package)
- Fresh domain setup required — SPF/DKIM/DMARC DNS records as user_setup step

**Email Content & Branding**
- Full estimate breakdown in email body — cost range (P50/P80), cost categories, timeline, assumptions
- Tenant-branded email template — uses contractor's white-label branding (logo, colors) from widget config
- Project-specific subject line — e.g., "How did [Project Name] go?"
- Magic link CTA button in email body

**Feedback Form Layout**
- Single-page form — all fields visible at once, no stepped wizard
- Original estimate summary displayed above the form fields — customer can compare while entering actuals
- Tenant-branded form page — matches contractor's white-label branding end-to-end (widget → email → form)
- After submission: simple thank-you message with contractor name ("Thanks for sharing your experience with [Contractor Name]!")

**Feedback Data Fields**
- Actual cost (numeric input)
- Actual timeline (numeric input)
- Overall rating: 1-5 stars
- Discrepancy reason: scope-focused enum with primary + optional secondary selection
  - Scope changed
  - Unforeseen issues
  - Timeline pressure
  - Vendor/material costs
  - Client changes
  - Estimate was accurate
- Optional free-text comment field for additional context

### Claude's Discretion
- Token generation implementation details (secrets.token_urlsafe format, hash algorithm already specified in requirements)
- Two-step GET/POST validation flow (specified in FEED-03)
- Expired/used link message copy and layout
- Email HTML template implementation approach
- Form field validation rules and error messages
- Mobile responsive behavior

### Deferred Ideas (OUT OF SCOPE)
- Automated email drip for non-responders — explicitly out of scope (CAN-SPAM, v2 FAUTO-02)
- Contractor notification after customer submits feedback — v2 (FAUTO-01)
- Per-tenant sending domains — too complex for v1.1, single efOfX domain sufficient
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FEED-01 | Email infrastructure setup — transactional provider, SPF/DKIM/DMARC configuration | Resend Python SDK v2.23.0 verified; DNS record pattern documented |
| FEED-02 | Magic link token generation (secrets.token_urlsafe) with SHA-256 hashed storage and 72h TTL | Pattern matches existing auth_service.py verification token flow exactly; MongoDB TTL index via expireAfterSeconds=0 on expires_at field |
| FEED-03 | Two-step token validation — idempotent GET renders form, POST consumes token | Security pattern verified; GET sets opened_at, POST deletes token_hash document |
| FEED-04 | Contextualized feedback email with estimate context and estimate range summary + CTA | Resend SDK resend.Emails.send() with HTML; run in FastAPI BackgroundTasks; Jinja2-style f-string template or string template |
| FEED-05 | Customer feedback form with structured fields (actual_cost, actual_timeline, rating, discrepancy reason enum) | React + Tailwind in widget or standalone page; star rating via pure CSS/React state |
| FEED-06 | Feedback document storage with immutable estimate snapshot and reference class linkage | Extend existing FeedbackService pattern; immutable snapshot stored in feedback doc at submit time |
| FEED-07 | Graceful token states — valid (form), expired (friendly message), used (thank you) | Three-state enum on token document; GET endpoint returns different HTML/JSON per state |
</phase_requirements>

## Summary

Phase 7 adds a complete feedback loop: contractor triggers an email to the customer, the customer clicks a magic link, fills in actual costs and outcomes, and the submission is stored with an immutable estimate snapshot for Phase 8 calibration. No customer account is required.

The email provider is locked to **Resend** (Python SDK `resend` v2.23.0). The SDK is synchronous — it must be dispatched via FastAPI's `BackgroundTasks` (or `run_in_threadpool`) to avoid blocking the async event loop. The core token security pattern — `secrets.token_urlsafe(32)` raw token, SHA-256 hash stored in MongoDB, MongoDB TTL index via `expires_at` field — is **already used verbatim in `app/services/auth_service.py`** for email verification tokens. The feedback magic link follows the exact same pattern with a 72-hour TTL.

The feedback form is a new standalone HTML page (not inside the widget shadow DOM) served from the FastAPI backend using `HTMLResponse` or a Jinja2 template. It uses the existing `BrandingConfig` tenant-branding data to apply contractor colors and logo. The feedback document stored in MongoDB embeds an immutable snapshot of the `EstimationOutput` at submission time — the stored data is not affected by later estimate changes.

**Primary recommendation:** Reuse the existing verification token pattern from `auth_service.py` for magic links (same code shape), use Resend SDK with `BackgroundTasks` for email dispatch, serve the feedback form as a FastAPI Jinja2 HTML response with tenant branding injected server-side, and store the feedback document with an embedded `estimate_snapshot` field.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `resend` | 2.23.0 (latest as of 2026-02-23) | Transactional email sending | Locked provider decision; official Python SDK |
| `motor` | 3.3.2 (already installed) | MongoDB TTL index for token expiry | Already the project's async DB driver |
| `secrets` (stdlib) | Python 3.13 stdlib | Cryptographically secure token generation | Stdlib; already used in auth_service.py |
| `hashlib` (stdlib) | Python 3.13 stdlib | SHA-256 hashing of raw token for DB storage | Stdlib; already used in auth_service.py |
| `Jinja2` | via `fastapi[all]` or standalone | HTML email template and feedback form page | FastAPI official template approach |
| `starlette.concurrency.run_in_threadpool` | via FastAPI | Run synchronous Resend SDK in async context | FastAPI recommended pattern for sync SDKs |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `fastapi.BackgroundTasks` | built-in | Dispatch email send after HTTP response | All email sends in this phase |
| `pydantic` | 2.11.7 (already installed) | FeedbackSubmission request validation | All new API request bodies |
| `datetime` / `timezone` | Python 3.13 stdlib | expires_at calculation, opened_at timestamp | Token TTL, event timestamps |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Resend SDK (sync) | httpx async directly | Would require managing Resend REST API auth headers; SDK already provides typed params |
| Jinja2 HTML template | Inline f-string HTML | f-strings are brittle for complex HTML; Jinja2 gives block structure, auto-escaping |
| MongoDB TTL index (expireAfterSeconds=0) | Background cleanup job | TTL index is background-thread-free and already used for verification_tokens and refresh_tokens in this project |
| Stand-alone feedback form page | Widget shadow DOM embed | Form needs a standalone URL to work as a magic link destination; shadow DOM is for embedded chat widget |

**Installation:**
```bash
# In apps/efofx-estimate — add to pyproject.toml dependencies
pip install resend
# Jinja2 + python-multipart already in project; confirm jinja2 in requirements.txt
```

## Architecture Patterns

### Recommended Project Structure

```
apps/efofx-estimate/
├── app/
│   ├── api/
│   │   ├── feedback_email.py   # POST /feedback/request-email (auth required, triggers email)
│   │   └── feedback_form.py    # GET/POST /feedback/form/{token} (public, no auth)
│   ├── models/
│   │   └── feedback.py         # extend: FeedbackMagicLink, FeedbackSubmission (NEW fields)
│   ├── services/
│   │   ├── feedback_email_service.py  # FeedbackEmailService (Resend SDK wrapper)
│   │   └── feedback_service.py        # extend: store_feedback_with_snapshot()
│   └── templates/
│       ├── feedback_email.html         # Jinja2 HTML email template
│       └── feedback_form.html          # Jinja2 HTML page for customer form
```

New MongoDB collection: `feedback_tokens` (one TTL-indexed document per magic link)

### Pattern 1: Magic Link Token Generation (matches auth_service.py verification token pattern)

**What:** Generate raw token → SHA-256 hash → store hash in MongoDB with expires_at → email raw token in URL.

**When to use:** Any one-time-use secret where storage of raw value would be insecure.

**Example:**
```python
# Source: derived from app/services/auth_service.py lines 55-59
import secrets
import hashlib
from datetime import datetime, timedelta, timezone

MAGIC_LINK_TTL_HOURS = 72

def generate_magic_link_token() -> tuple[str, str, datetime]:
    """Generate raw token, SHA-256 hash, and 72h expiry.

    Returns (raw_token, token_hash, expires_at).
    Store token_hash in MongoDB. Email raw_token in URL.
    SHA-256 enables O(1) MongoDB lookup without bcrypt overhead.
    """
    raw = secrets.token_urlsafe(32)
    token_hash = hashlib.sha256(raw.encode()).hexdigest()
    expires_at = datetime.now(timezone.utc) + timedelta(hours=MAGIC_LINK_TTL_HOURS)
    return raw, token_hash, expires_at
```

MongoDB document shape for `feedback_tokens` collection:
```python
{
    "token_hash": "<sha256-hex>",      # indexed, unique — lookup key
    "tenant_id": "<uuid>",              # for branding lookup on GET
    "estimation_session_id": "<str>",  # to fetch estimate context on GET
    "customer_email": "<str>",          # for display in form header
    "expires_at": datetime,             # TTL index on this field (expireAfterSeconds=0)
    "opened_at": datetime | None,       # SET on first GET (idempotent)
    "used_at": datetime | None,         # SET on POST (marks as consumed)
    "created_at": datetime,
}
```

### Pattern 2: Two-Step GET/POST Token Validation (FEED-03)

**What:** GET endpoint is idempotent — renders form without consuming token. POST endpoint consumes token (sets `used_at`, deletes or marks done). Email security scanners follow GET links and do not POST forms, so they cannot burn the token.

**When to use:** Any magic link where email scanner protection is required.

**Example:**
```python
# Source: architecture derived from STATE.md decision + CONTEXT.md specifics
@router.get("/feedback/form/{token}")
async def get_feedback_form(token: str) -> HTMLResponse:
    """Idempotent GET — renders form or appropriate state message.

    Token states:
    - not found OR expired: render expired.html fragment
    - used_at is set:       render thank-you.html fragment
    - valid:                set opened_at (if not set), render form.html

    Does NOT set used_at. Email scanners can hit this URL without harm.
    """
    token_hash = hashlib.sha256(token.encode()).hexdigest()
    db = get_database()
    token_doc = await db["feedback_tokens"].find_one({"token_hash": token_hash})

    if token_doc is None:
        return HTMLResponse(render_template("expired_link.html"))

    now = datetime.now(timezone.utc)
    expires_at = token_doc["expires_at"]
    if expires_at.tzinfo is None:
        expires_at = expires_at.replace(tzinfo=timezone.utc)

    if now > expires_at:
        return HTMLResponse(render_template("expired_link.html"))

    if token_doc.get("used_at") is not None:
        return HTMLResponse(render_template("already_submitted.html", ...))

    # Idempotent: set opened_at only once
    if token_doc.get("opened_at") is None:
        await db["feedback_tokens"].update_one(
            {"token_hash": token_hash},
            {"$set": {"opened_at": now}},
        )

    # Load estimate + branding for display
    ...
    return HTMLResponse(render_template("feedback_form.html", ...))


@router.post("/feedback/form/{token}")
async def submit_feedback_form(token: str, form: FeedbackSubmission) -> HTMLResponse:
    """POST consumes token — stores feedback + estimate snapshot.

    Validates token state, stores feedback document with immutable estimate
    snapshot, then marks token as used (sets used_at). Subsequent GETs show
    thank-you state.
    """
    token_hash = hashlib.sha256(token.encode()).hexdigest()
    # ... validate token state (same checks as GET) ...
    # ... insert feedback document with estimate_snapshot ...
    await db["feedback_tokens"].update_one(
        {"token_hash": token_hash},
        {"$set": {"used_at": now}},
    )
    return HTMLResponse(render_template("thank_you.html", ...))
```

### Pattern 3: Resend SDK in FastAPI Background Task

**What:** Resend Python SDK v2.23.0 is synchronous (blocking HTTP call). Run it via FastAPI `BackgroundTasks` with `run_in_threadpool` wrapper or as a plain `def` background task so it does not block the async event loop.

**When to use:** Any endpoint that triggers email dispatch.

**Example:**
```python
# Source: Resend official docs (resend.com/docs/send-with-python) + FastAPI BackgroundTasks pattern
import resend
from starlette.concurrency import run_in_threadpool
from fastapi import BackgroundTasks

async def send_feedback_email_background(params: resend.Emails.SendParams) -> None:
    """Send feedback email via Resend, running sync SDK in thread pool."""
    try:
        await run_in_threadpool(resend.Emails.send, params)
    except Exception as exc:
        logger.error("Failed to send feedback email: %s", exc)
        # Do not re-raise — email failure must not block the trigger response


@router.post("/feedback/request-email/{session_id}")
async def request_feedback_email(
    session_id: str,
    background_tasks: BackgroundTasks,
    tenant: Tenant = Depends(get_current_tenant),
) -> dict:
    """Contractor triggers feedback email for a delivered estimate."""
    raw, token_hash, expires_at = generate_magic_link_token()
    # ... store token doc in MongoDB ...
    # ... build HTML email with estimate context + magic link ...
    magic_link = f"{settings.APP_BASE_URL}/feedback/form/{raw}"

    params: resend.Emails.SendParams = {
        "from": "feedback@efofx.com",
        "to": [customer_email],
        "subject": f"How did {project_name} go?",
        "html": build_feedback_email_html(estimate, tenant, magic_link),
    }
    background_tasks.add_task(send_feedback_email_background, params)
    return {"message": "Feedback email queued"}
```

### Pattern 4: Feedback Document with Immutable Estimate Snapshot (FEED-06)

**What:** At POST submission time, embed a copy of the `EstimationOutput` fields into the feedback document. This snapshot is never updated — later changes to the estimate session do not affect it.

```python
# New fields on feedback document
{
    "_id": ...,
    "tenant_id": ...,
    "estimation_session_id": ...,
    "reference_class_id": ...,            # linkage for Phase 8 calibration
    "actual_cost": 62000.0,
    "actual_timeline": 10,
    "rating": 4,
    "discrepancy_reason_primary": "scope_changed",
    "discrepancy_reason_secondary": "vendor_material_costs",  # optional
    "comment": "...",
    "estimate_snapshot": {                # IMMUTABLE — copied at submission time
        "total_cost_p50": 58000,
        "total_cost_p80": 72000,
        "timeline_weeks_p50": 8,
        "timeline_weeks_p80": 11,
        "cost_breakdown": [...],
        "assumptions": [...],
        "confidence_score": 82.0,
    },
    "submitted_at": datetime,
    "schema_version": 1,
}
```

### Anti-Patterns to Avoid

- **Storing raw token in MongoDB:** Store only the SHA-256 hash. If the DB is compromised, raw tokens cannot be replayed. Already established in auth_service.py.
- **Consuming the token on GET:** Email scanners fetch GET URLs. Consuming on GET burns the token before the customer sees it. Only POST consumes.
- **Using `time.sleep()` or `asyncio.sleep()` for rate-limiting email:** Use `BackgroundTasks` for fire-and-forget; don't add artificial delays.
- **Calling `resend.Emails.send()` inside an `async def` directly:** The sync SDK blocks the event loop. Always wrap with `run_in_threadpool` or use a `def` background task.
- **Storing mutable reference to EstimationSession in feedback doc:** The session can be updated. Copy the fields verbatim as `estimate_snapshot` at submission time.
- **Soft-deleting tokens to mark "used":** Setting `used_at` (not deleting) enables the thank-you state to be re-rendered on subsequent GET visits to the same link.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Secure random token generation | Custom UUID/random | `secrets.token_urlsafe(32)` | stdlib, cryptographically secure, already used in project |
| Token hashing | Custom HMAC | `hashlib.sha256(raw.encode()).hexdigest()` | Already the project pattern; O(1) lookup |
| Email sending | SMTP via fastapi-mail | `resend` Python SDK | Locked decision; Resend handles SPF/DKIM automatically on verified domain |
| Token TTL expiry | Background cleanup cron | MongoDB TTL index (`expireAfterSeconds=0` on `expires_at`) | Already used for `verification_tokens` and `refresh_tokens` in this project |
| HTML email rendering | Complex template system | Python f-string or Jinja2 template | Sufficient for one email type; inline CSS required for email clients |
| Feedback form page rendering | React SPA with API calls | FastAPI `TemplateResponse` (Jinja2) | Public URL, no auth, server-renders branding — simpler than a separate SPA |

**Key insight:** The token pattern (generate → hash → store → TTL expire) is already fully implemented in `app/services/auth_service.py` for email verification. The magic link implementation is structurally identical — copy the pattern, change the TTL from 24h to 72h and the collection name from `verification_tokens` to `feedback_tokens`.

## Common Pitfalls

### Pitfall 1: Email Scanner Burns the Token (FEED-03)
**What goes wrong:** A one-step magic link (GET /feedback/form/{token} consumes the token) means corporate email security systems that visit URLs in incoming emails will consume the token before the customer opens the email. The customer gets an "expired link" page.
**Why it happens:** Many enterprise mail gateways proactively visit all URLs in emails to check for phishing/malware.
**How to avoid:** Two-step validation: GET is idempotent (only sets `opened_at`, never `used_at`). Only form POST consumes the token. This is a locked architecture decision from STATE.md and CONTEXT.md.
**Warning signs:** Testing from corporate email addresses shows expired link immediately; testing from Gmail/personal works.

### Pitfall 2: Resend SDK Blocks the Event Loop
**What goes wrong:** Calling `resend.Emails.send(params)` directly inside an `async def` endpoint freezes all concurrent requests for the duration of the HTTP call (typically 100-500ms but can be seconds on failure).
**Why it happens:** `resend.Emails.send()` v2.23.0 is synchronous — it makes a blocking HTTPS call to Resend's API.
**How to avoid:** Wrap in `await run_in_threadpool(resend.Emails.send, params)` or use `background_tasks.add_task()` with a `def` (non-async) wrapper function. The BackgroundTasks approach (fire-and-forget after response) is preferred since email delivery latency should not block the contractor's UI.
**Warning signs:** Endpoint response times spike to 200-500ms when Resend is reachable; spike to 30s+ when Resend is unreachable.

### Pitfall 3: Timezone-Naive expires_at Comparison
**What goes wrong:** `datetime.utcnow()` returns a timezone-naive datetime. Comparing it to a timezone-aware `datetime.now(timezone.utc)` raises a TypeError in Python 3.11+.
**Why it happens:** MongoDB stores datetimes without timezone info by default; Python 3.11+ enforces aware/naive comparison strictness.
**How to avoid:** Always use `datetime.now(timezone.utc)` for all timestamps. When reading `expires_at` from MongoDB, add `if expires_at.tzinfo is None: expires_at = expires_at.replace(tzinfo=timezone.utc)`. This exact pattern is already in `auth_service.py` lines 222-225.
**Warning signs:** `TypeError: can't compare offset-naive and offset-aware datetimes` in test or production logs.

### Pitfall 4: Missing Inline CSS in Email HTML
**What goes wrong:** Email clients (Gmail, Outlook, Apple Mail) strip `<style>` tags and external CSS from HTML emails. Branding colors applied via CSS classes are lost.
**Why it happens:** Email clients only render inline CSS (`style="..."` attributes).
**How to avoid:** All styling in the email HTML template must use inline CSS. Brand colors (from `BrandingConfig.primary_color`, `accent_color`) are interpolated into inline `style="..."` attributes in the Jinja2 template. Do not use Tailwind classes in the email HTML template.
**Warning signs:** Email renders with no colors/styles when opened in Gmail.

### Pitfall 5: Feedback Form Submitted Twice (Double-Submit)
**What goes wrong:** Customer double-clicks the submit button, sending two POST requests. Two feedback documents are inserted for one magic link.
**Why it happens:** Form submission with no debounce or server-side idempotency check.
**How to avoid:** Token is marked `used_at` on first POST. Second POST finds `used_at` is set and returns the thank-you page immediately (same as GET after submission). The token state check before writing is the idempotency gate.
**Warning signs:** Duplicate feedback documents in MongoDB for the same `estimation_session_id`.

### Pitfall 6: Feedback Form Page Not Mobile Responsive
**What goes wrong:** Customer opens magic link on mobile (common for email-driven CTAs); form is unusable.
**Why it happens:** Server-rendered Jinja2 template with fixed-width containers.
**How to avoid:** Use `<meta name="viewport" content="width=device-width, initial-scale=1">` and fluid-width layout. Mobile-responsive feedback form is listed under Claude's Discretion — plan should include a responsive layout. This is not optional given email → mobile context.
**Warning signs:** Manual test: open magic link on iPhone; form has horizontal scroll or tiny text.

### Pitfall 7: TTL Index Not Created for feedback_tokens
**What goes wrong:** Magic link tokens never expire — 72-hour TTL has no effect.
**Why it happens:** TTL index must be explicitly created in `create_indexes()` in `mongodb.py`. Simply storing `expires_at` in the document does nothing without the index.
**How to avoid:** Add to `create_indexes()`:
```python
await db["feedback_tokens"].create_index("expires_at", expireAfterSeconds=0)
await db["feedback_tokens"].create_index("token_hash", unique=True)
await db["feedback_tokens"].create_index([("tenant_id", 1), ("estimation_session_id", 1)])
```
**Warning signs:** Old tokens still in `feedback_tokens` collection days after creation; no MongoDB background TTL thread logs.

## Code Examples

Verified patterns from official sources and existing codebase:

### Resend SDK: Send HTML Email
```python
# Source: resend.com/docs/send-with-python (verified 2026-03-01, SDK v2.23.0)
import resend

resend.api_key = os.environ["RESEND_API_KEY"]

params: resend.Emails.SendParams = {
    "from": "feedback@efofx.com",
    "to": ["customer@example.com"],
    "subject": "How did Your Pool Project go?",
    "html": "<strong>Your feedback matters!</strong>",
    # Optional extras:
    # "reply_to": ["support@efofx.com"],
    # "tags": [{"name": "tenant_id", "value": "abc123"}],
}

email: resend.Emails.SendResponse = resend.Emails.send(params)
# email.id is the Resend message ID
```

### Token Generation (mirrors auth_service.py)
```python
# Source: app/services/auth_service.py lines 55-59 (adapted for magic links)
import secrets
import hashlib
from datetime import datetime, timedelta, timezone

def generate_magic_link_token() -> tuple[str, str, datetime]:
    """Return (raw_token, sha256_hash, expires_at)."""
    raw = secrets.token_urlsafe(32)
    token_hash = hashlib.sha256(raw.encode()).hexdigest()
    expires_at = datetime.now(timezone.utc) + timedelta(hours=72)
    return raw, token_hash, expires_at
```

### MongoDB TTL Index for feedback_tokens
```python
# Source: app/db/mongodb.py create_indexes() pattern (already in project)
# Add to create_indexes() function:
await db["feedback_tokens"].create_index(
    "expires_at", expireAfterSeconds=0
)
await db["feedback_tokens"].create_index("token_hash", unique=True)
await db["feedback_tokens"].create_index(
    [("tenant_id", 1), ("estimation_session_id", 1)]
)
```

### Token State Check (GET endpoint)
```python
# Source: derived from auth_service.py verify_email() pattern (lines 211-246)
async def resolve_token_state(token: str) -> tuple[str, dict | None]:
    """Returns (state, token_doc) where state is 'valid' | 'expired' | 'used' | 'not_found'."""
    token_hash = hashlib.sha256(token.encode()).hexdigest()
    db = get_database()
    token_doc = await db["feedback_tokens"].find_one({"token_hash": token_hash})

    if token_doc is None:
        return "not_found", None

    now = datetime.now(timezone.utc)
    expires_at = token_doc["expires_at"]
    if expires_at.tzinfo is None:
        expires_at = expires_at.replace(tzinfo=timezone.utc)

    if now > expires_at:
        return "expired", token_doc

    if token_doc.get("used_at") is not None:
        return "used", token_doc

    return "valid", token_doc
```

### FastAPI Jinja2 Template Response
```python
# Source: FastAPI official templates docs (fastapi.tiangolo.com/advanced/templates/)
from fastapi.templating import Jinja2Templates
from fastapi.responses import HTMLResponse

templates = Jinja2Templates(directory="app/templates")

@router.get("/feedback/form/{token}", response_class=HTMLResponse)
async def get_feedback_form(request: Request, token: str) -> HTMLResponse:
    state, token_doc = await resolve_token_state(token)
    if state in ("not_found", "expired"):
        return templates.TemplateResponse(
            "feedback_expired.html",
            {"request": request},
        )
    if state == "used":
        branding = await load_branding(token_doc["tenant_id"])
        return templates.TemplateResponse(
            "feedback_submitted.html",
            {"request": request, "branding": branding},
        )
    # state == "valid" — set opened_at and render form
    ...
    return templates.TemplateResponse(
        "feedback_form.html",
        {"request": request, "estimate": estimate_context, "branding": branding, "token": token},
    )
```

### Discrepancy Reason Enum
```python
# New enum for feedback model
from enum import Enum

class DiscrepancyReason(str, Enum):
    SCOPE_CHANGED = "scope_changed"
    UNFORESEEN_ISSUES = "unforeseen_issues"
    TIMELINE_PRESSURE = "timeline_pressure"
    VENDOR_MATERIAL_COSTS = "vendor_material_costs"
    CLIENT_CHANGES = "client_changes"
    ESTIMATE_WAS_ACCURATE = "estimate_was_accurate"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| fastapi-mail (SMTP) | Resend SDK (transactional API) | Phase 7 decision | No SMTP server management; SPF/DKIM handled by Resend on domain verification |
| Resend SDK sync call in `async def` | `run_in_threadpool(resend.Emails.send, ...)` | Current FastAPI best practice | Avoids event loop blocking |
| Magic link GET consumes token | GET idempotent; only POST consumes | 2023+ industry pattern | Prevents email scanner from burning token |
| Feedback form as React SPA | FastAPI Jinja2 server-rendered HTML | Phase 7 architecture decision | No frontend build pipeline needed; branding injected server-side |

**Deprecated/outdated:**
- SMTP-based email via fastapi-mail: project already has `fastapi-mail` in `pyproject.toml` (used for consultation notifications in DEBT-04), but for feedback email specifically, Resend is the locked provider. Do not add a second SMTP pathway.
- Plain-text verification token storage (storing raw token in DB): already deprecated in this project in favor of SHA-256 hash storage (auth_service.py).

## Open Questions

1. **Where is the "Request Feedback" button in the contractor UI?**
   - What we know: Contractor triggers manually via a button on a "delivered estimate" view
   - What's unclear: The contractor UI is the widget (React) or a separate contractor dashboard app? No contractor dashboard SPA exists yet — the API routes handle contractor auth.
   - Recommendation: Expose the trigger as a new authenticated API endpoint (`POST /api/v1/feedback/request-email/{session_id}`). The contractor calls this from whatever UI they use. Plan 07-03 should document this as an API endpoint, not a UI button — the UI button is out of scope for this phase unless a contractor dashboard exists.

2. **Customer email stored where for feedback email delivery?**
   - What we know: The customer email address must be available when the contractor triggers the feedback request
   - What's unclear: Is `customer_email` stored in the `EstimationSession` or `LeadCapture` record? Reviewing `LeadCapture` model: it has `email` field. The `EstimationSession` does not have customer_email.
   - Recommendation: The trigger endpoint should accept `customer_email` as a required body field (contractor provides it), or look it up from the most recent `widget_leads` record for the session. Plan should clarify — the simplest approach is requiring contractor to supply it in the trigger request body since they know who to send it to.

3. **Feedback form served from efofx-estimate FastAPI or separate app?**
   - What we know: `APP_BASE_URL` is already a settings field. The feedback form is a public HTML page.
   - What's unclear: Does the feedback form live in `apps/efofx-estimate/app/templates/` served by FastAPI, or in a separate Next.js/React app?
   - Recommendation: Serve from `apps/efofx-estimate` FastAPI using Jinja2 templates. This avoids a new app, keeps everything in one deploy unit, and matches the widget branding endpoint pattern. The form is simple enough that Jinja2 + inline CSS suffices.

4. **Resend API key configuration scope**
   - What we know: `RESEND_API_KEY` needs to be added to `Settings` in `app/core/config.py`
   - What's unclear: Should it be optional (graceful degradation like SMTP_USERNAME) or required?
   - Recommendation: Make it optional with a warning log (same pattern as `SMTP_USERNAME` in `auth_service.py`). Log the magic link URL to the dev console when Resend is not configured, enabling local dev without email setup. FEED-01 user_setup step provisions the real key.

## Validation Architecture

> nyquist_validation is NOT in .planning/config.json (no `workflow.nyquist_validation` key). Checking config: `{"mode": "yolo", "depth": "standard", "parallelization": true, "commit_docs": true, "model_profile": "balanced", "workflow": {"research": true, "plan_check": true, "verifier": true}}`. Key `nyquist_validation` is absent — skip this section per instructions.

## Sources

### Primary (HIGH confidence)
- `apps/efofx-estimate/app/services/auth_service.py` — Token generation pattern (secrets.token_urlsafe, hashlib.sha256, verification_tokens TTL pattern)
- `apps/efofx-estimate/app/db/mongodb.py` — create_indexes() TTL index pattern (expireAfterSeconds=0 on expires_at)
- `apps/efofx-estimate/app/core/config.py` — Settings pattern for optional email credentials
- `apps/efofx-estimate/pyproject.toml` — Current dependency versions (pydantic 2.11.7, motor 3.3.2, fastapi 0.116.1)
- `apps/efofx-estimate/app/models/widget.py` — BrandingConfig structure for tenant branding
- `https://resend.com/docs/send-with-python` — Resend Python SDK API (verified 2026-03-01): `resend.Emails.send(params)` with `resend.Emails.SendParams` typed dict
- `https://pypi.org/project/resend/` — Current version 2.23.0 (released 2026-02-23)
- `https://resend.com/docs/api-reference/emails/send-email` — Full Resend API parameters

### Secondary (MEDIUM confidence)
- `https://resend.com/fastapi` — FastAPI integration example (synchronous `resend.Emails.send()` in route handlers; no async variant documented)
- `https://www.mongodb.com/docs/manual/core/index-ttl/` — MongoDB TTL index documentation confirming expireAfterSeconds=0 behavior
- FastAPI official docs on BackgroundTasks and `run_in_threadpool` from `starlette.concurrency`
- `https://resend.com/docs/dashboard/domains/dmarc` — Resend DMARC configuration guidance

### Tertiary (LOW confidence)
- Multiple WebSearch results confirming that Resend SDK v2.23.0 has no async variant (GitHub issue #122 from Sep 2024 requesting async support — unresolved at time of research)
- WebSearch results confirming two-step GET/POST magic link pattern for email scanner protection is industry standard

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Resend SDK version verified on PyPI; all other libraries already in pyproject.toml
- Architecture: HIGH — Token pattern directly mirrors existing auth_service.py code in this codebase
- Pitfalls: HIGH — Email scanner issue is documented architecture decision in STATE.md; other pitfalls directly observed from existing code patterns
- Resend async support: LOW — SDK appears synchronous only; confirmed via GitHub issue but no official statement found. Mitigation (run_in_threadpool) is well-established FastAPI pattern.

**Research date:** 2026-03-01
**Valid until:** 2026-04-01 (Resend SDK is active; check for async support additions at planning time)
