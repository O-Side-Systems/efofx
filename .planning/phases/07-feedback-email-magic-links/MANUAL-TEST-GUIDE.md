# Phase 7: Feedback Email & Magic Links — Manual Test Guide

## Prerequisites

1. **Server running:** `cd apps/efofx-estimate && python -m uvicorn app.main:app --reload --port 8000`
2. **MongoDB running** with data (tenants, at least one estimate session)
3. **No RESEND_API_KEY needed** — dev mode logs the magic link URL to console

## Setup: Get Auth Token

```bash
# Login to get JWT token
curl -s -X POST http://localhost:8000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "<YOUR_TENANT_EMAIL>", "password": "<YOUR_PASSWORD>"}' | jq .

# Save the access_token for subsequent calls
export TOKEN="<access_token from response>"
```

Or if you have an API key (`sk_live_...`), use that instead:
```bash
export TOKEN="sk_live_..."
```

## Setup: Find a Session ID

```bash
# List estimation sessions for your tenant
curl -s http://localhost:8000/api/v1/estimates \
  -H "Authorization: Bearer $TOKEN" | jq '.[0].session_id'

export SESSION_ID="<session_id from response>"
```

---

## Test 1: Trigger Feedback Email (Dev Mode)

**What to do:**
```bash
curl -s -X POST "http://localhost:8000/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "customer@example.com", "project_name": "Kitchen Renovation"}' | jq .
```

**Expected response:**
```json
{
  "message": "Feedback email queued",
  "token_hash": "a1b2c3..."  // 64-char hex string
}
```

**Check server console output** — you should see a log line like:
```
WARNING - RESEND_API_KEY not configured — skipping feedback email to customer@example.com. Magic link for dev testing: http://localhost:8000/feedback/form/<RAW_TOKEN>
```

**Copy the magic link URL from the log** — you'll need it for the next tests.

```bash
export MAGIC_LINK="http://localhost:8000/feedback/form/<RAW_TOKEN_FROM_LOG>"
```

**Also verify 404 for bad session:**
```bash
curl -s -X POST "http://localhost:8000/api/v1/feedback/request-email/nonexistent_session" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "customer@example.com", "project_name": "Test"}' | jq .
# Expected: {"detail": "Estimation session not found"} with 404 status
```

**Also verify 401 without auth:**
```bash
curl -s -X POST "http://localhost:8000/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "customer@example.com", "project_name": "Test"}' | jq .
# Expected: 401 Unauthorized
```

---

## Test 2: Email Template Renders Correctly

**What to do:**

You can't see the actual email without Resend, but you can verify the template renders by checking the HTML output directly. Use the feedback form's GET endpoint (Test 3) which uses the same estimate context rendering, or render the template manually:

```bash
# Quick check: fetch the rendered email HTML via a Python one-liner
cd apps/efofx-estimate
python -c "
from jinja2 import Environment, FileSystemLoader
env = Environment(loader=FileSystemLoader('app/templates'), autoescape=True)
t = env.get_template('feedback_email.html')
html = t.render(
    company_name='Acme Builders',
    logo_url=None,
    primary_color='#2563eb',
    accent_color='#1d4ed8',
    project_name='Kitchen Renovation',
    total_cost_p50=45000,
    total_cost_p80=62000,
    timeline_weeks_p50=8,
    timeline_weeks_p80=12,
    cost_breakdown=[
        {'category': 'Materials', 'p50_cost': 20000, 'p80_cost': 28000},
        {'category': 'Labor', 'p50_cost': 25000, 'p80_cost': 34000},
    ],
    assumptions=['Standard materials', 'No permit delays'],
    magic_link_url='http://localhost:8000/feedback/form/test-token',
)
print(html)
" > /tmp/feedback_email_preview.html
open /tmp/feedback_email_preview.html   # macOS
```

**Verify in browser:**
- [ ] Header shows company name on colored background
- [ ] Cost range displayed: `$45,000 — $62,000`
- [ ] Timeline displayed: `8 — 12 weeks`
- [ ] Cost breakdown table with Materials and Labor rows
- [ ] Assumptions bullet list
- [ ] Blue "Share Your Feedback" CTA button
- [ ] Footer: "This link expires in 72 hours." and "Powered by efOfX"
- [ ] All styling is inline (view source — no `<style>` blocks, only `style="..."` attributes)
- [ ] Fallback link text below button

---

## Test 3: Magic Link Opens Feedback Form

**What to do:**

Open the magic link URL from Test 1 in your browser:
```
http://localhost:8000/feedback/form/<RAW_TOKEN>
```

**Verify:**
- [ ] Page loads — no error, no login prompt
- [ ] Header shows tenant company name with branding color
- [ ] Estimate summary card above the form shows P50/P80 cost range and timeline
- [ ] Cost breakdown table (if the estimate session has breakdown data)
- [ ] Assumptions list (if present)
- [ ] Page title: "Project Feedback - {Company Name}"

---

## Test 4: Feedback Form Has All Required Fields

**On the same page from Test 3, verify these form fields exist:**

- [ ] **Actual Total Cost** — numeric input, required, placeholder "e.g. 58000"
- [ ] **Actual Timeline (weeks)** — numeric input, required, placeholder "e.g. 10"
- [ ] **Overall Rating** — 5-star selector (click stars, they should highlight)
- [ ] **Primary Reason** — dropdown with 6 options:
  - Scope changed
  - Unforeseen issues
  - Timeline pressure
  - Vendor/material costs
  - Client changes
  - Estimate was accurate
- [ ] **Secondary Reason (optional)** — same dropdown with "None" as default
- [ ] **Additional Feedback (optional)** — textarea, 2000 char max
- [ ] **Submit Feedback** button at bottom

---

## Test 5: Submit Feedback Successfully

**What to do:**

Fill out the form from Test 3/4:
- Actual Cost: `55000`
- Actual Timeline: `10`
- Rating: click 4 stars
- Primary Reason: select "Unforeseen issues"
- Secondary Reason: leave as "None" or select one
- Comment: "Some unexpected plumbing work needed"

Click **Submit Feedback**.

**Verify:**
- [ ] Redirected to thank-you page
- [ ] Page shows: "Thanks for sharing your experience with {Company Name}!"
- [ ] Page shows: "Your feedback helps improve future estimates."
- [ ] "Powered by efOfX" footer

**Verify data was stored in MongoDB:**
```bash
# Check feedback collection
mongosh efofx_estimate --eval "db.feedback.find().sort({submitted_at: -1}).limit(1).pretty()"
```

**Verify the stored document has:**
- [ ] `actual_cost: 55000`
- [ ] `actual_timeline: 10`
- [ ] `rating: 4`
- [ ] `discrepancy_reason_primary: "unforeseen_issues"`
- [ ] `comment: "Some unexpected plumbing work needed"`
- [ ] `estimate_snapshot` object with `total_cost_p50`, `total_cost_p80`, `timeline_weeks_p50`, `timeline_weeks_p80`, `cost_breakdown`, `assumptions`, `confidence_score`
- [ ] `tenant_id` and `estimation_session_id` populated
- [ ] `submitted_at` timestamp

---

## Test 6: Used Token Shows Thank-You Page

**What to do:**

Open the **same magic link URL** from Test 1 again in your browser (after having submitted in Test 5).

**Verify:**
- [ ] You see the thank-you page — NOT the form again
- [ ] Message: "Thanks for sharing your experience with {Company Name}!"

**Also test via curl:**
```bash
curl -s "$MAGIC_LINK" | grep -o "Thanks for sharing"
# Should output: "Thanks for sharing"
```

---

## Test 7: Expired or Invalid Token Shows Friendly Message

**What to do:**

Visit a garbage token URL in your browser:
```
http://localhost:8000/feedback/form/this-is-not-a-real-token
```

**Verify:**
- [ ] Page loads (not a 500 error or raw stack trace)
- [ ] Shows: "This feedback link has expired or is no longer available."
- [ ] Shows: "Feedback links are valid for 72 hours after they are sent."
- [ ] Shows: "If you need to provide feedback, please ask your contractor for a new link."
- [ ] "Powered by efOfX" footer

**Optional — test actual expiry** (requires modifying the DB):
```bash
# Manually expire a token by setting expires_at to the past
mongosh efofx_estimate --eval "
db.feedback_tokens.updateOne(
  {token_hash: '<TOKEN_HASH_FROM_TEST_1>'},
  {\$set: {expires_at: new Date('2020-01-01')}}
)"
```
Then create a new magic link (Test 1 again) and visit it — should show form. Expire it in DB — should show expired page.

---

## Test 8: Dev Mode Graceful Degradation

**What to do:**

Ensure `RESEND_API_KEY` is NOT set in your `.env` file (or is empty).

Trigger a feedback email (same as Test 1):
```bash
curl -s -X POST "http://localhost:8000/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "test@example.com", "project_name": "Dev Test"}' | jq .
```

**Verify:**
- [ ] Response is `200` with `{"message": "Feedback email queued", "token_hash": "..."}`
- [ ] Server log shows WARNING (not ERROR): `RESEND_API_KEY not configured — skipping feedback email to test@example.com. Magic link for dev testing: http://localhost:8000/feedback/form/...`
- [ ] The magic link URL in the log is valid and opens the form
- [ ] No crash, no exception

---

## Test 9: Mobile-Responsive Form

**What to do:**

Open the magic link from Test 1 in Chrome DevTools mobile simulator:
1. Open Chrome DevTools (F12)
2. Toggle device toolbar (Ctrl+Shift+M / Cmd+Shift+M)
3. Select "iPhone SE" or "iPhone 12 Pro" preset
4. Reload the page

**Verify:**
- [ ] Form fills the viewport width — no horizontal scrolling
- [ ] Estimate summary stats stack vertically (single column on small screens)
- [ ] All form fields are full-width and tappable
- [ ] Star rating is usable (stars are large enough to tap)
- [ ] Dropdowns open properly
- [ ] Submit button is full-width
- [ ] Text is readable without zooming
- [ ] Also check the expired and thank-you pages on mobile

---

## Quick Smoke Test Script

Run all API-level tests in one go:

```bash
#!/bin/bash
BASE="http://localhost:8000"

echo "=== 1. Trigger feedback email ==="
RESULT=$(curl -s -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "smoke@test.com", "project_name": "Smoke Test"}')
echo "$RESULT" | jq .
TOKEN_HASH=$(echo "$RESULT" | jq -r '.token_hash')
echo "Token hash: $TOKEN_HASH"
echo ""

echo "=== 2. Get magic link from server logs ==="
echo "(Check server console for the magic link URL)"
echo ""

echo "=== 3. Test invalid token ==="
curl -s "$BASE/feedback/form/invalid-token-abc123" | grep -o "expired or is no longer available" && echo "PASS" || echo "FAIL"
echo ""

echo "=== 4. Test 404 for bad session ==="
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/v1/feedback/request-email/nonexistent" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "x@x.com", "project_name": "X"}')
[ "$STATUS" = "404" ] && echo "404 PASS" || echo "FAIL: got $STATUS"
echo ""

echo "=== 5. Test 401 without auth ==="
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "x@x.com", "project_name": "X"}')
[ "$STATUS" = "401" ] && echo "401 PASS" || echo "FAIL: got $STATUS"
```

---

## Test Results Checklist

| # | Test | Result |
|---|------|--------|
| 1 | Trigger feedback email endpoint | |
| 2 | Email template renders correctly | |
| 3 | Magic link opens feedback form | |
| 4 | Form has all required fields | |
| 5 | Submit feedback successfully | |
| 6 | Used token shows thank-you | |
| 7 | Expired/invalid token shows friendly msg | |
| 8 | Dev mode graceful degradation | |
| 9 | Mobile-responsive form | |
