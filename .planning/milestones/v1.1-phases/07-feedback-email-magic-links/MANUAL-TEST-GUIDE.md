# Phase 7: Feedback Email & Magic Links — Manual Test Guide

## Step 0: Fix Rate Limiter & Start Server

```bash
# Disable rate limiting (avoids Redis/Valkey dependency)
cd apps/efofx-estimate
sed -i '' 's/RATE_LIMIT_ENABLED=true/RATE_LIMIT_ENABLED=false/' .env

# Start the server (use venv python)
.venv/bin/python -m uvicorn app.main:app --reload --port 8000
```

Keep this terminal open. Open a **new terminal** for the test commands below.

---

## Step 1: Bootstrap — Register Tenant + Seed Session

Run this entire block in one go. It registers a test tenant, seeds an estimation session with realistic data, and exports all the variables you need.

```bash
cd apps/efofx-estimate
BASE="http://localhost:8000"

# --- Register a new test tenant ---
REG=$(curl -s -X POST "$BASE/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "company_name": "Acme Builders",
    "email": "test@acmebuilders.com",
    "password": "TestPass123!"
  }')
echo "Registration response:"
echo "$REG" | python3 -m json.tool

export TENANT_ID=$(echo "$REG" | python3 -c "import sys,json; print(json.load(sys.stdin)['tenant_id'])")
export TOKEN=$(echo "$REG" | python3 -c "import sys,json; print(json.load(sys.stdin)['api_key'])")

echo ""
echo "TENANT_ID=$TENANT_ID"
echo "TOKEN=$TOKEN"

# --- Seed a completed estimation session with estimate data ---
export SESSION_ID="test-session-$(date +%s)"

.venv/bin/python -c "
import asyncio, os, sys
from motor.motor_asyncio import AsyncIOMotorClient
from datetime import datetime, timezone

uri = os.environ.get('MONGO_URI', '')
if not uri:
    # Read from .env
    with open('.env') as f:
        for line in f:
            if line.startswith('MONGO_URI='):
                uri = line.strip().split('=', 1)[1]
                break

client = AsyncIOMotorClient(uri)
db = client['estimator']

tenant_id = os.environ['TENANT_ID']
session_id = os.environ['SESSION_ID']

async def seed():
    await db['estimates'].insert_one({
        'tenant_id': tenant_id,
        'session_id': session_id,
        'status': 'completed',
        'description': 'Full kitchen renovation including cabinets, countertops, and appliances',
        'region': 'SoCal - Coastal',
        'reference_class': 'kitchen_renovation',
        'confidence_threshold': 0.7,
        'prompt_version': '1.0.0',
        'result': None,
        'chat_messages': [],
        'images': [],
        'created_at': datetime.now(timezone.utc),
        'updated_at': datetime.now(timezone.utc),
        'completed_at': datetime.now(timezone.utc),
        'estimation_output': {
            'total_cost_p50': 45000,
            'total_cost_p80': 62000,
            'timeline_weeks_p50': 8,
            'timeline_weeks_p80': 12,
            'cost_breakdown': [
                {'category': 'Materials', 'p50_cost': 18000, 'p80_cost': 25000, 'percentage_of_total': 0.40},
                {'category': 'Labor', 'p50_cost': 22000, 'p80_cost': 30000, 'percentage_of_total': 0.49},
                {'category': 'Permits & Fees', 'p50_cost': 5000, 'p80_cost': 7000, 'percentage_of_total': 0.11}
            ],
            'adjustment_factors': [
                {'name': 'Coastal premium', 'multiplier': 1.12, 'reason': 'SoCal coastal labor rates'},
                {'name': 'Full renovation scope', 'multiplier': 1.05, 'reason': 'Cabinets + counters + appliances'}
            ],
            'confidence_score': 78,
            'assumptions': [
                'Standard mid-range materials (no luxury upgrades)',
                'No structural modifications required',
                'Existing plumbing and electrical in good condition',
                'Permits approved without delay'
            ],
            'summary': 'A full kitchen renovation in the SoCal coastal area typically runs between \$45,000 and \$62,000.'
        }
    })
    print(f'Seeded session: {session_id}')

asyncio.run(seed())
client.close()
"

echo ""
echo "=========================================="
echo "  Bootstrap complete!"
echo "  SESSION_ID=$SESSION_ID"
echo "  TOKEN=$TOKEN"
echo "=========================================="
```

---

## Test 1: Trigger Feedback Email (Dev Mode)

```bash
# Trigger feedback email — look for magic link URL in server console
RESULT=$(curl -s -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "customer@example.com", "project_name": "Kitchen Renovation"}')
echo "$RESULT" | python3 -m json.tool

# !! NOW: copy the magic link URL from the SERVER console log !!
# It looks like: http://localhost:8000/feedback/form/<long-token>
# Paste it here:
export MAGIC_LINK="<paste magic link URL from server log>"
```

**Verify in server console** — you should see:
```
WARNING - RESEND_API_KEY not configured — skipping feedback email to customer@example.com.
Magic link for dev testing: http://localhost:8000/feedback/form/xxxxxxxx
```

```bash
# Verify 404 for nonexistent session
curl -s -o /dev/null -w "Status: %{http_code} (expect 404)\n" \
  -X POST "$BASE/api/v1/feedback/request-email/nonexistent_session" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "x@x.com", "project_name": "X"}'

# Verify 401 without auth
curl -s -o /dev/null -w "Status: %{http_code} (expect 401)\n" \
  -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "x@x.com", "project_name": "X"}'
```

---

## Test 2: Email Template Renders Correctly

```bash
cd apps/efofx-estimate
.venv/bin/python -c "
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
        {'category': 'Materials', 'p50_cost': 18000, 'p80_cost': 25000},
        {'category': 'Labor', 'p50_cost': 22000, 'p80_cost': 30000},
        {'category': 'Permits & Fees', 'p50_cost': 5000, 'p80_cost': 7000},
    ],
    assumptions=['Standard mid-range materials', 'No structural modifications', 'Existing plumbing OK', 'Permits approved without delay'],
    magic_link_url='http://localhost:8000/feedback/form/test-token',
)
print(html)
" > /tmp/feedback_email_preview.html
open /tmp/feedback_email_preview.html
```

**Check in browser:**
- Header with "Acme Builders" on blue background
- Cost range: $45,000 — $62,000
- Timeline: 8 — 12 weeks
- 3-row breakdown table (Materials, Labor, Permits & Fees)
- 4 assumption bullets
- Blue "Share Your Feedback" CTA button
- Footer: "This link expires in 72 hours." / "Powered by efOfX"
- All inline styles (view source — no `<style>` blocks)

---

## Test 3: Magic Link Opens Feedback Form

Open in browser (paste your MAGIC_LINK):
```bash
open "$MAGIC_LINK"
```

**Or verify via curl:**
```bash
curl -s "$MAGIC_LINK" | grep -c "feedback-form" && echo "PASS: form rendered" || echo "FAIL: form not found"
```

**Check in browser:**
- Page loads without login prompt
- Header shows "Acme Builders" with blue branding
- Estimate summary: $45,000 – $62,000 cost range, 8 – 12 weeks
- Cost breakdown table (Materials, Labor, Permits & Fees)
- Assumptions list

---

## Test 4: Feedback Form Has All Required Fields

**On the same page from Test 3, verify these fields:**

```bash
# Automated check — each field present in HTML
curl -s "$MAGIC_LINK" | python3 -c "
import sys
html = sys.stdin.read()
checks = [
    ('actual_cost input',     'name=\"actual_cost\"' in html),
    ('actual_timeline input', 'name=\"actual_timeline\"' in html),
    ('rating input',          'name=\"rating\"' in html),
    ('primary reason select', 'name=\"discrepancy_reason_primary\"' in html),
    ('secondary reason',      'name=\"discrepancy_reason_secondary\"' in html),
    ('comment textarea',      'name=\"comment\"' in html),
    ('submit button',         'type=\"submit\"' in html or 'Submit' in html),
    ('scope_changed option',  'scope_changed' in html),
    ('unforeseen_issues',     'unforeseen_issues' in html),
    ('estimate_was_accurate', 'estimate_was_accurate' in html),
]
for name, ok in checks:
    print(f'  {\"PASS\" if ok else \"FAIL\"}: {name}')
print(f'\n{sum(1 for _,ok in checks if ok)}/{len(checks)} checks passed')
"
```

---

## Test 5: Submit Feedback Successfully

```bash
# Extract raw token from magic link URL
RAW_TOKEN=$(echo "$MAGIC_LINK" | sed 's|.*/feedback/form/||')

# Submit the form via POST (form-encoded, not JSON)
curl -s -X POST "$BASE/feedback/form/$RAW_TOKEN" \
  -d "actual_cost=55000" \
  -d "actual_timeline=10" \
  -d "rating=4" \
  -d "discrepancy_reason_primary=unforeseen_issues" \
  -d "discrepancy_reason_secondary=" \
  -d "comment=Some unexpected plumbing work needed" \
  | python3 -c "
import sys
html = sys.stdin.read()
if 'Thanks for sharing' in html or 'thank' in html.lower():
    print('PASS: Thank-you page rendered')
else:
    print('FAIL: Expected thank-you page')
    print(html[:500])
"
```

**Verify data in MongoDB:**
```bash
cd apps/efofx-estimate
.venv/bin/python -c "
import asyncio, os
from motor.motor_asyncio import AsyncIOMotorClient

with open('.env') as f:
    for line in f:
        if line.startswith('MONGO_URI='):
            uri = line.strip().split('=', 1)[1]; break

client = AsyncIOMotorClient(uri)
db = client['estimator']

async def check():
    doc = await db['feedback'].find_one(sort=[('submitted_at', -1)])
    if not doc:
        print('FAIL: No feedback document found')
        return
    checks = [
        ('actual_cost == 55000',         doc.get('actual_cost') == 55000),
        ('actual_timeline == 10',        doc.get('actual_timeline') == 10),
        ('rating == 4',                  doc.get('rating') == 4),
        ('primary reason',              doc.get('discrepancy_reason_primary') == 'unforeseen_issues'),
        ('comment present',             doc.get('comment') == 'Some unexpected plumbing work needed'),
        ('estimate_snapshot exists',    'estimate_snapshot' in doc and doc['estimate_snapshot'] is not None),
        ('snapshot has total_cost_p50', doc.get('estimate_snapshot', {}).get('total_cost_p50') == 45000),
        ('snapshot has cost_breakdown', len(doc.get('estimate_snapshot', {}).get('cost_breakdown', [])) == 3),
        ('tenant_id set',              doc.get('tenant_id') is not None),
        ('session_id set',             doc.get('estimation_session_id') is not None),
        ('submitted_at set',           doc.get('submitted_at') is not None),
    ]
    for name, ok in checks:
        print(f'  {\"PASS\" if ok else \"FAIL\"}: {name}')
    print(f'\n{sum(1 for _,ok in checks if ok)}/{len(checks)} checks passed')

asyncio.run(check())
client.close()
"
```

---

## Test 6: Used Token Shows Thank-You Page

```bash
# Re-visit the same magic link — should show thank-you, NOT the form
curl -s "$MAGIC_LINK" | python3 -c "
import sys
html = sys.stdin.read()
if 'Thanks for sharing' in html or 'thank' in html.lower():
    print('PASS: Used token shows thank-you page (not form)')
elif 'actual_cost' in html:
    print('FAIL: Form is showing again — token was not consumed')
else:
    print('UNCLEAR: Check page manually')
    print(html[:300])
"
```

---

## Test 7: Expired / Invalid Token Shows Friendly Message

```bash
# Visit a garbage token URL
curl -s "$BASE/feedback/form/this-is-not-a-real-token-abc123" | python3 -c "
import sys
html = sys.stdin.read()
checks = [
    ('expired message',   'expired' in html.lower() or 'no longer available' in html.lower()),
    ('72 hours mention',  '72 hours' in html),
    ('not a raw error',   'Traceback' not in html and '500' not in html),
    ('powered by efOfX',  'efOfX' in html or 'Powered by' in html),
]
for name, ok in checks:
    print(f'  {\"PASS\" if ok else \"FAIL\"}: {name}')
print(f'\n{sum(1 for _,ok in checks if ok)}/{len(checks)} checks passed')
"
```

---

## Test 8: Dev Mode Graceful Degradation

```bash
# Trigger another feedback email (no RESEND_API_KEY set)
curl -s -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "devtest@example.com", "project_name": "Dev Mode Test"}' \
  | python3 -c "
import sys, json
resp = json.load(sys.stdin)
checks = [
    ('200 with message',  resp.get('message') == 'Feedback email queued'),
    ('token_hash present', len(resp.get('token_hash', '')) == 64),
]
for name, ok in checks:
    print(f'  {\"PASS\" if ok else \"FAIL\"}: {name}')
print()
print('Now check the SERVER console for:')
print('  WARNING - RESEND_API_KEY not configured — skipping ...')
print('  Magic link for dev testing: http://localhost:8000/feedback/form/...')
print()
print('PASS if: warning logged (not error), magic link URL shown, no crash')
"
```

---

## Test 9: Mobile-Responsive Form

This one requires a browser. Trigger a fresh magic link and open it in Chrome DevTools:

```bash
# Trigger a fresh token for mobile testing
MOBILE_RESULT=$(curl -s -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "mobile@example.com", "project_name": "Mobile Test"}')
echo "$MOBILE_RESULT" | python3 -m json.tool
echo ""
echo "Copy the magic link URL from server console, then:"
echo "1. Open in Chrome"
echo "2. DevTools (Cmd+Option+I) → Toggle Device Toolbar (Cmd+Shift+M)"
echo "3. Select 'iPhone SE' or 'iPhone 12 Pro'"
echo ""
echo "Check: form fills viewport, fields stack vertically, stars are tappable,"
echo "       submit button is full-width, no horizontal scrolling"
```

---

## Full Smoke Test Script (Tests 1, 7, 8 in one go)

Copy/paste this entire block — runs all API-level checks automatically:

```bash
BASE="http://localhost:8000"

echo "=== Smoke Test: Phase 7 Feedback Email & Magic Links ==="
echo ""
PASS=0; FAIL=0

# 1. Trigger feedback email
echo "--- Test 1: Trigger feedback email ---"
R=$(curl -s -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"customer_email": "smoke@test.com", "project_name": "Smoke Test"}')
MSG=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin).get('message',''))" 2>/dev/null)
if [ "$MSG" = "Feedback email queued" ]; then echo "  PASS: email queued"; PASS=$((PASS+1)); else echo "  FAIL: $R"; FAIL=$((FAIL+1)); fi

# 2. 404 for bad session
echo "--- Test 2: 404 for bad session ---"
S=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/v1/feedback/request-email/nonexistent" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"customer_email": "x@x.com", "project_name": "X"}')
if [ "$S" = "404" ]; then echo "  PASS: 404"; PASS=$((PASS+1)); else echo "  FAIL: got $S"; FAIL=$((FAIL+1)); fi

# 3. 401 without auth
echo "--- Test 3: 401 without auth ---"
S=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Content-Type: application/json" -d '{"customer_email": "x@x.com", "project_name": "X"}')
if [ "$S" = "401" ]; then echo "  PASS: 401"; PASS=$((PASS+1)); else echo "  FAIL: got $S"; FAIL=$((FAIL+1)); fi

# 4. Invalid token → expired page
echo "--- Test 4: Invalid token shows expired page ---"
BODY=$(curl -s "$BASE/feedback/form/bogus-token-12345")
if echo "$BODY" | grep -qi "expired\|no longer available"; then echo "  PASS: expired page"; PASS=$((PASS+1)); else echo "  FAIL: unexpected response"; FAIL=$((FAIL+1)); fi

# 5. Dev mode — no crash, returns success
echo "--- Test 5: Dev mode graceful (no RESEND_API_KEY) ---"
R2=$(curl -s -X POST "$BASE/api/v1/feedback/request-email/$SESSION_ID" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"customer_email": "dev@test.com", "project_name": "Dev Test"}')
MSG2=$(echo "$R2" | python3 -c "import sys,json; print(json.load(sys.stdin).get('message',''))" 2>/dev/null)
if [ "$MSG2" = "Feedback email queued" ]; then echo "  PASS: dev mode ok"; PASS=$((PASS+1)); else echo "  FAIL: $R2"; FAIL=$((FAIL+1)); fi

echo ""
echo "=========================================="
echo "  Results: $PASS passed, $FAIL failed"
echo "=========================================="
echo ""
echo "Remaining manual checks:"
echo "  - Open magic link from server log in browser (Tests 3-6)"
echo "  - Fill and submit feedback form (Test 5)"
echo "  - Re-visit same link → should show thank-you (Test 6)"
echo "  - Check mobile responsive in DevTools (Test 9)"
```

---

## Cleanup (Optional)

Re-enable rate limiting when done:
```bash
cd apps/efofx-estimate
sed -i '' 's/RATE_LIMIT_ENABLED=false/RATE_LIMIT_ENABLED=true/' .env
```

Remove test data:
```bash
cd apps/efofx-estimate
.venv/bin/python -c "
import asyncio, os
from motor.motor_asyncio import AsyncIOMotorClient

with open('.env') as f:
    for line in f:
        if line.startswith('MONGO_URI='):
            uri = line.strip().split('=', 1)[1]; break

client = AsyncIOMotorClient(uri)
db = client['estimator']

async def cleanup():
    r1 = await db['tenants'].delete_many({'email': 'test@acmebuilders.com'})
    r2 = await db['estimates'].delete_many({'session_id': {'\\$regex': '^test-session-'}})
    r3 = await db['feedback'].delete_many({'comment': 'Some unexpected plumbing work needed'})
    r4 = await db['feedback_tokens'].delete_many({})
    print(f'Deleted: {r1.deleted_count} tenants, {r2.deleted_count} sessions, {r3.deleted_count} feedback, {r4.deleted_count} tokens')

asyncio.run(cleanup())
client.close()
"
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
