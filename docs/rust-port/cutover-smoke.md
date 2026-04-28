# Cutover Smoke Test

**Phase:** 3.4
**Purpose:** End-to-end manual exercise of the Rust core driving both
clients (`apps/efofx-widget` and `apps/efofx-dashboard`) from a clean
Mongo state. Run this before declaring Phase 3 done.

## Prereqs

- MongoDB running locally on `mongodb://localhost:27017`.
- Supabase project configured (`qzdgqepcdaoyigrgtlzv` per
  `.env.example`); a real test user (email + password) provisioned in
  the Supabase dashboard.
- Resend test-mode key (`re_...`) optional — without it the consultation
  flow logs instead of sending.
- Node 20+, npm 10+, Rust stable.

## 0. Reset state

```bash
mongosh "mongodb://localhost:27017/efofx" --eval "db.dropDatabase()"
```

## 1. Boot the Rust core

```bash
cd apps/efofx-core
cp .env.example .env   # fill in EFOFX_CRYPTO__MASTER_KEY at minimum
cargo run --bin efofx-core
```

Wait for `listening on 127.0.0.1:8080`.

## 2. Boot the dashboard, sign in, mint an API key

```bash
cd apps/efofx-dashboard
cp .env.example .env   # fill in VITE_SUPABASE_ANON_KEY
npm install
npm run dev
```

In a browser:

1. Visit `http://localhost:5173/login`.
2. Sign in with the Supabase test user.
3. First authenticated call hits `GET /v1/calibration/metrics`, which
   triggers `TenantResolver::resolve_or_provision`. Mongo now has a
   tenant document keyed by the JWT `sub` claim.
4. Mint a widget API key:
   ```bash
   curl -X POST http://localhost:8080/v1/me/api-keys:rotate \
     -H "Authorization: Bearer <copy-from-supabase-session>"
   ```
   Copy the returned `api_key` (starts with `sk_live_`). The token
   lives in the Supabase session — DevTools → Application → Local
   Storage → `sb-<project>-auth-token`.

The dashboard at `/` should render the empty-state "below threshold"
panel — no calibration data yet. Sign-out button in the header should
clear the session and redirect to `/login`.

## 3. Boot the widget against the same core

```bash
cd apps/efofx-widget
cp .env.example .env   # set VITE_DEMO_API_KEY to the sk_live_… from step 2
npm install
npm run dev
```

In a browser, visit the dev server's `test-embed.html` route
(typically `http://localhost:5174/test-embed.html`). Confirm:

- **Branding fetch** — widget header renders the tenant name (no console
  errors about `GET /v1/widget/branding/...`).
- **First chat message** — typing and sending posts
  `POST /v1/chat/sessions` (creates the session). Network tab shows
  201 with the new `session_id`.
- **Second chat message** — posts to
  `POST /v1/chat/sessions/{id}/messages`, NOT a fresh `/sessions`.
- **Generate estimate** — UI button fires
  `POST /v1/chat/sessions/{id}:generate-estimate`. SSE stream:
  - `thinking` events render in the streaming UI
  - `estimate` event delivers full output JSON
  - unnamed `data:` frames append narrative text
  - `done` event includes `routing_tags` (additive, ignored client-side)
- **Lead capture** — fill the lead form, submit. `POST /v1/widget/leads`
  returns 201.
- **Consultation request** — submit the consultation CTA.
  `POST /v1/widget/consultations` returns 201. With Resend wired, the
  test-mode email arrives at the `from_address` you configured.
- **Analytics event** — fires `POST /v1/widget/events` on widget mount /
  interaction. Confirm 204.

All requests must include `x-api-key: sk_live_...` (NOT
`Authorization: Bearer ...`). Bearer is reserved for Supabase JWTs.

## 4. Calibration round-trip

Back in the dashboard:

1. Refresh `/`. The calibration metrics panel should now show 1 outcome
   (still below threshold by default).
2. Date-range filter renders without errors —
   `GET /v1/calibration/metrics?date_range=all` returns 200.
3. Trend page renders without errors —
   `GET /v1/calibration/trend?months=12` returns 200.

## 5. No client-side workflow rules

Spot-check that no client gates a server-side rule. None of the
following should appear in `apps/efofx-widget/src/` or
`apps/efofx-dashboard/src/`:

- Chat readiness scoring or stage gating
- Estimate budget enforcement
- Magic-link / feedback expiry checks
- Consultation status transitions

If any client file enforces such a rule, file it as Phase 4 cleanup
and remove before merging Phase 3.

## Definition of Done (3.4)

- [ ] Steps 0–4 all complete cleanly from a fresh `dropDatabase`.
- [ ] No `4xx`/`5xx` in either client's network tab during the golden
      path (excepting deliberate validation tests if you run any).
- [ ] No client file duplicates server workflow rules.
