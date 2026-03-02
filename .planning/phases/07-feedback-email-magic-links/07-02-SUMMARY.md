---
phase: 07-feedback-email-magic-links
plan: 02
subsystem: feedback-magic-links
tags: [magic-links, tokens, security, sha256, mongodb, tdd]
dependency_graph:
  requires: []
  provides:
    - MagicLinkService (generate, create, resolve_state, mark_opened, consume)
    - DiscrepancyReason enum
    - FeedbackMagicLink, FeedbackSubmission, FeedbackDocument, EstimateSnapshot models
    - feedback_tokens MongoDB indexes (TTL + unique + compound)
  affects:
    - apps/efofx-estimate/app/models/feedback.py
    - apps/efofx-estimate/app/core/constants.py
    - apps/efofx-estimate/app/db/mongodb.py
tech_stack:
  added:
    - hashlib.sha256 for token hashing
    - secrets.token_urlsafe(32) for token generation
  patterns:
    - SHA-256 hash storage (never raw token in DB)
    - MongoDB TTL index on expires_at for auto-cleanup
    - Idempotent mark_opened via {opened_at: None} filter
    - consume returns bool (False if already used)
key_files:
  created:
    - apps/efofx-estimate/app/services/magic_link_service.py
    - apps/efofx-estimate/tests/services/test_magic_link_service.py
  modified:
    - apps/efofx-estimate/app/models/feedback.py
    - apps/efofx-estimate/app/core/constants.py
    - apps/efofx-estimate/app/db/mongodb.py
decisions:
  - "Token hash is SHA-256 hex digest stored in DB; raw token only returned to caller for email embedding"
  - "mark_opened uses {opened_at: None} filter — second call is no-op at DB level (idempotent)"
  - "consume returns False when used_at already set (modified_count=0), True when consumed (modified_count=1)"
  - "resolve_token_state handles timezone-naive datetimes from MongoDB by coercing to UTC"
metrics:
  duration: "~4 minutes"
  completed: "2026-03-02T14:20:10Z"
  tasks_completed: 2
  files_modified: 5
  tests_added: 17
---

# Phase 7 Plan 02: Magic Link Service and Feedback Models Summary

**One-liner:** SHA-256 hashed magic link tokens with full lifecycle (generate/create/resolve/open/consume), feedback models, and MongoDB TTL indexes for 72h auto-expiry.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Define feedback models, DiscrepancyReason enum, and DB constants | ebfdf82 | feedback.py, constants.py, mongodb.py |
| 2 | Create MagicLinkService with TDD (token generate, resolve, open, consume) | 7686556 | magic_link_service.py, test_magic_link_service.py |

## What Was Built

### Feedback Models (app/models/feedback.py)

New models appended below existing feedback classes:

- **DiscrepancyReason** (str, Enum): 6 values — scope_changed, unforeseen_issues, timeline_pressure, vendor_material_costs, client_changes, estimate_was_accurate
- **EstimateSnapshot**: Immutable copy of EstimationOutput fields at submission time (not a reference — prevents later estimate changes from corrupting stored feedback context)
- **FeedbackMagicLink**: MongoDB document shape for feedback_tokens — stores token_hash (SHA-256), never raw token. TTL via expires_at field.
- **FeedbackSubmission**: Request body for form POST with validation (actual_cost > 0, actual_timeline > 0, rating 1-5, primary discrepancy reason)
- **FeedbackDocument**: Full feedback document with estimate snapshot embedded

### DB Constants (app/core/constants.py)

Added `"FEEDBACK_TOKENS": "feedback_tokens"` to DB_COLLECTIONS.

### MongoDB Indexes (app/db/mongodb.py)

Added to create_indexes() for feedback_tokens collection:
- TTL index on expires_at (expireAfterSeconds=0) — auto-deletes after 72h
- Unique index on token_hash — prevents duplicate token collisions
- Compound index on (tenant_id, estimation_session_id) — efficient per-session token lookup

### MagicLinkService (app/services/magic_link_service.py)

Full token lifecycle:

| Method | Description |
|--------|-------------|
| `generate_token()` | Static. Returns (raw_token, token_hash, expires_at). SHA-256 hash of raw token. 72h TTL. |
| `hash_token(raw)` | Static. Returns SHA-256 hex digest for DB lookup. |
| `create_magic_link(tenant_id, session_id, email, project_name)` | Inserts doc with token_hash only. Returns (raw, hash). |
| `resolve_token_state(raw_token)` | Returns ('valid'\|'expired'\|'used'\|'not_found', doc). Handles naive datetimes. |
| `mark_opened(raw_token)` | Sets opened_at via {opened_at: None} filter — idempotent. |
| `consume(raw_token)` | Sets used_at via {used_at: None} filter. Returns False if already used. |

### Unit Tests (tests/services/test_magic_link_service.py)

17 tests organized by class:

- **TestGenerateToken** (5): tuple types, URL-safe string, 64-char hex, 72h expiry, SHA-256 correctness
- **TestCreateMagicLink** (2): stores hash not raw, returns (raw, hash)
- **TestResolveTokenState** (5): valid/expired/used/not_found/naive datetime handling
- **TestMarkOpened** (2): opened_at filter, idempotent second call
- **TestConsume** (3): returns True on first consume, False on already-used, used_at None filter

All 17 tests pass with mocked MongoDB using unittest.mock.AsyncMock.

## Decisions Made

1. **SHA-256 only in DB**: Raw token is returned to caller for email embedding but never persisted — database compromise cannot replay tokens.

2. **Idempotent mark_opened**: Uses `{token_hash: ..., opened_at: None}` as filter — if opened_at is already set, MongoDB finds no matching doc and does nothing. No application-level check needed.

3. **consume returns bool**: `modified_count > 0` cleanly maps to success/already-used without additional reads.

4. **Naive datetime handling**: MongoDB may return naive datetimes; resolve_token_state coerces to UTC before comparison — prevents TypeError on datetime subtraction.

5. **MAGIC_LINK_TTL_HOURS = 72**: Module-level constant (not buried in code) — easy to adjust for future plans.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Added naive datetime handling in resolve_token_state**
- **Found during:** Task 2, GREEN phase
- **Issue:** Plan's resolve_token_state implementation already included this handling, correctly anticipating MongoDB returning naive datetimes — included as specified
- **Fix:** Implemented as specified in the plan (`if expires_at.tzinfo is None: expires_at = expires_at.replace(tzinfo=timezone.utc)`)
- **Files modified:** apps/efofx-estimate/app/services/magic_link_service.py

None other — plan executed exactly as written with 17 tests (17 tests vs plan's "10+").

## Self-Check

### Files Created/Modified

- [x] apps/efofx-estimate/app/services/magic_link_service.py — created
- [x] apps/efofx-estimate/tests/services/test_magic_link_service.py — created
- [x] apps/efofx-estimate/app/models/feedback.py — modified
- [x] apps/efofx-estimate/app/core/constants.py — modified
- [x] apps/efofx-estimate/app/db/mongodb.py — modified

### Commits

- ebfdf82: feat(07-02): add feedback models, DiscrepancyReason enum, and DB constants
- 7686556: feat(07-02): implement MagicLinkService with full token lifecycle and tests

## Self-Check: PASSED
