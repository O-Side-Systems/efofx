# Phase 7: Feedback Email & Magic Links - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Customers submit actual project costs and outcomes via a time-limited email magic link after an estimate. No customer login required. The contractor manually triggers feedback requests, and the submitted data is stored with an immutable estimate snapshot for calibration in Phase 8.

</domain>

<decisions>
## Implementation Decisions

### Email Trigger & Sending
- Manual "Request Feedback" button — contractor clicks on a delivered estimate to trigger the email
- No automatic sending — contractor controls timing and which customers to ask
- Single efOfX sending domain (e.g., feedback@efofx.com) — not per-tenant sender addresses
- Resend as transactional email provider via official Python SDK (`resend` package)
- Fresh domain setup required — SPF/DKIM/DMARC DNS records as user_setup step

### Email Content & Branding
- Full estimate breakdown in email body — cost range (P50/P80), cost categories, timeline, assumptions
- Tenant-branded email template — uses contractor's white-label branding (logo, colors) from widget config
- Project-specific subject line — e.g., "How did [Project Name] go?"
- Magic link CTA button in email body

### Feedback Form Layout
- Single-page form — all fields visible at once, no stepped wizard
- Original estimate summary displayed above the form fields — customer can compare while entering actuals
- Tenant-branded form page — matches contractor's white-label branding end-to-end (widget → email → form)
- After submission: simple thank-you message with contractor name ("Thanks for sharing your experience with [Contractor Name]!")

### Feedback Data Fields
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

</decisions>

<specifics>
## Specific Ideas

- End-to-end white-label experience: widget → estimate delivery → feedback email → feedback form all use the same contractor branding
- Email scanner protection: two-step GET (renders form) / POST (consumes token) prevents email security crawlers from burning the magic link
- Discrepancy reasons are scope-focused to give contractors actionable calibration data — "why was my estimate off?" has a clear answer

</specifics>

<deferred>
## Deferred Ideas

- Automated email drip for non-responders — explicitly out of scope (CAN-SPAM, v2 FAUTO-02)
- Contractor notification after customer submits feedback — v2 (FAUTO-01)
- Per-tenant sending domains — too complex for v1.1, single efOfX domain sufficient

</deferred>

---

*Phase: 07-feedback-email-magic-links*
*Context gathered: 2026-03-01*
