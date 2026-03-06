---
status: testing
phase: 07-feedback-email-magic-links
source: [07-01-SUMMARY.md, 07-02-SUMMARY.md, 07-03-SUMMARY.md, 07-04-SUMMARY.md]
started: 2026-03-02T15:00:00Z
updated: 2026-03-02T15:00:00Z
---

## Current Test

number: 1
name: Trigger Feedback Email Endpoint
expected: |
  POST /api/v1/feedback/request-email/{session_id} with a valid contractor JWT returns 200 with {"message": "Feedback email queued", "token_hash": "..."}.
  The endpoint creates a magic link token and queues an email via BackgroundTasks.
awaiting: user response

## Tests

### 1. Trigger Feedback Email Endpoint
expected: POST /api/v1/feedback/request-email/{session_id} with a valid contractor JWT and customer_email in request body returns 200 with a success message and token_hash. Invalid session_id returns 404.
result: [pending]

### 2. Email Template Renders with Estimate Context
expected: The feedback email HTML contains the project name in the subject line, the P50/P80 cost range, cost breakdown by category, timeline estimates, assumptions list, tenant branding (logo, company name, primary color), and a "Share Your Feedback" CTA button linking to /feedback/form/{token}. All CSS is inline (no style blocks).
result: [pending]

### 3. Magic Link Opens Feedback Form
expected: GET /feedback/form/{token} with a valid unexpired token renders a branded HTML form page showing the original estimate summary (P50/P80 range, cost breakdown) above the form fields. The page uses tenant branding (logo, colors, company name). No login required.
result: [pending]

### 4. Feedback Form Has All Required Fields
expected: The feedback form contains: actual cost (numeric input), actual timeline (numeric input), 1-5 star rating selector, primary discrepancy reason dropdown (6 options: scope_changed, unforeseen_issues, timeline_pressure, vendor_material_costs, client_changes, estimate_was_accurate), optional secondary discrepancy reason dropdown, and optional free-text comment textarea.
result: [pending]

### 5. Submit Feedback Successfully
expected: POST /feedback/form/{token} with valid form data (actual cost, timeline, rating, discrepancy reason) shows a thank-you page with the contractor's company name. The feedback is stored as a FeedbackDocument with an immutable EstimateSnapshot copied at submission time.
result: [pending]

### 6. Used Token Shows Thank-You Page
expected: After successfully submitting feedback, revisiting the same magic link URL (GET /feedback/form/{token}) shows the thank-you page — not the form again. The token cannot be reused for a second submission.
result: [pending]

### 7. Expired or Invalid Token Shows Friendly Message
expected: Visiting /feedback/form/{token} with an expired or nonexistent token shows a friendly "this link has expired or is no longer available" message — not a raw error or blank page.
result: [pending]

### 8. Dev Mode Graceful Degradation
expected: When RESEND_API_KEY is not set (local dev), triggering a feedback email does not crash. Instead, the magic link URL is logged to console as a warning so the developer can manually open it for testing. The endpoint still returns success.
result: [pending]

### 9. Mobile-Responsive Form
expected: The feedback form page renders properly on a mobile viewport — fields stack vertically, text is readable, CTA button is full-width, and the form is usable without horizontal scrolling.
result: [pending]

## Summary

total: 9
passed: 0
issues: 0
pending: 9
skipped: 0

## Gaps

[none yet]
