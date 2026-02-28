---
phase: 03-llm-integration
plan: 03
subsystem: chat-service
tags: [chat, conversation, state-machine, scoping, mongodb, ttl, llm, context-extraction]

dependency_graph:
  requires:
    - 03-01 (LLMService — generate_response for follow-up questions, constructor injection pattern)
    - 03-02 (PromptService — get("scoping", "latest") for conversation prompt, get_version_string for session traceability)
  provides:
    - ChatService conversation state machine with multi-turn history
    - ScopingContext model tracking project_type, project_size, location, timeline, special_conditions
    - Readiness detection (rule-based: all 4 required fields populated)
    - Auto-trigger confirmation message when ready
    - MongoDB TTL index on chat_sessions.expires_at (expireAfterSeconds=0)
    - 33 unit tests covering full conversation flow
  affects:
    - 03-04 (narrative generator — ChatSession.is_ready=True signals readiness for narrative/estimate generation)

tech_stack:
  added:
    - re (regex patterns for size/timeline/location extraction)
  patterns:
    - Embedded message array in ChatSession (not separate collection)
    - ScopingContext populated incrementally — never overwrites existing fields
    - Rule-based readiness detection (field presence check, not LLM)
    - Error boundary: OpenAIError caught, conversation persisted, error message returned (not raised)
    - Explicit trigger phrases + short confirmation detection (ESTIMATE_TRIGGER_PHRASES, CONFIRMATION_WORDS sets)

key_files:
  created:
    - apps/efofx-estimate/tests/services/test_chat_service.py
  modified:
    - apps/efofx-estimate/app/models/chat.py
    - apps/efofx-estimate/app/services/chat_service.py
    - apps/efofx-estimate/app/db/mongodb.py
    - apps/efofx-estimate/app/api/routes.py

key_decisions:
  - "ScopingContext extraction uses keyword/regex patterns (not LLM) — fast, cost-free, good enough for readiness detection"
  - "Readiness requires all 4 fields: project_type, project_size, location, timeline — special_conditions is optional bonus"
  - "Conversation preserved on ANY error — session persisted before returning error response, never raises from send_message"
  - "Explicit trigger bypasses readiness check — user can request estimate at any time regardless of context completeness"
  - "Location extraction has two tiers: known region keywords (mapped to canonical Region values) + generic regex fallback"
  - "Auto-trigger appends confirmation phrase to LLM response — not a separate message, feels natural in conversation"

patterns_established:
  - "ScopingContext.is_ready() as the single readiness truth — checked after every context update"
  - "Never overwrite populated scoping context fields — incremental accumulation only"
  - "MongoDB upsert pattern: update_one with upsert=True for session persistence"

requirements_completed: [CHAT-01, CHAT-02, CHAT-03, CHAT-04, CHAT-05]

metrics:
  duration: 4 min
  completed_date: "2026-02-27"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 5
---

# Phase 3 Plan 03: Conversation Engine Summary

**LLM-powered project scoping state machine with keyword/regex context extraction, rule-based readiness detection, auto-trigger confirmation, and 24-hour TTL session storage in MongoDB.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-27T17:00:58Z
- **Completed:** 2026-02-27T17:04:58Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- ScopingContext model tracks 5 project detail fields with `is_ready()`, `missing_fields()`, and `populated_fields()` methods
- ChatService fully rewritten as conversation state machine: multi-turn history, LLM-powered follow-ups via PromptService scoping prompt, rule-based readiness detection
- 33 unit tests covering session creation, multi-turn accumulation, all 5 context extraction types, readiness transitions, explicit triggers, confirmation detection, and error preservation
- MongoDB TTL index on `chat_sessions.expires_at` with 24-hour session lifetime
- Conversation always preserved across LLM errors — no user progress lost

## Task Commits

1. **Task 1: Update chat models and add ScopingContext** - `fcadd22` (feat)
2. **Task 2: Rewrite ChatService, add TTL index, create tests** - `1da5098` (feat)

## Files Created/Modified

- `apps/efofx-estimate/app/models/chat.py` - Added ScopingContext, updated ChatSession (embedded messages, TTL), simplified ChatMessage, updated ChatRequest/ChatResponse, removed ChatMessageCreate/ChatSessionCreate
- `apps/efofx-estimate/app/services/chat_service.py` - Full rewrite: conversation state machine, LLM follow-up via PromptService, keyword/regex context extraction, readiness detection, auto-trigger, error preservation
- `apps/efofx-estimate/app/db/mongodb.py` - Added TTL index on chat_sessions.expires_at (expireAfterSeconds=0)
- `apps/efofx-estimate/app/api/routes.py` - Updated /chat/send to propagate HTTPException (for 402 passthrough)
- `apps/efofx-estimate/tests/services/test_chat_service.py` - 33 unit tests (new file)

## Decisions Made

- **Keyword/regex context extraction over LLM extraction**: Using pattern matching for ScopingContext updates is instantaneous and free. The LLM is still used for follow-up question generation — the scoping questions naturally guide users to provide the info we need. Context extraction is a bonus signal for readiness detection.
- **Two-tier location extraction**: First checks known region keywords (maps to canonical Region enum values like "NorCal - Bay Area"). Falls back to generic regex pattern ("in the Bay Area" -> "Bay Area") for locations not in the known list.
- **Conversation never raises from send_message**: All OpenAIError types are caught, a user-friendly error message is appended, and the session is persisted before returning. This is the locked decision: "Conversation is preserved across errors."
- **Auto-trigger appends to LLM response content**: When readiness transitions to True, the confirmation question is appended to the LLM's response rather than replacing it. This feels natural — the LLM might acknowledge the last answer and then we add the confirmation prompt.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## Next Phase Readiness

- ChatService complete — Plan 03-04 (narrative generator) can now use `session.is_ready=True` and `session.scoping_context` as inputs
- ChatSession.messages contains full conversation history for narrative generation context
- PromptService is wired and loaded at startup — narrative prompt will use the same pattern

---
*Phase: 03-llm-integration*
*Completed: 2026-02-27*

## Self-Check: PASSED

- FOUND: apps/efofx-estimate/app/models/chat.py
- FOUND: apps/efofx-estimate/app/services/chat_service.py
- FOUND: apps/efofx-estimate/app/db/mongodb.py
- FOUND: apps/efofx-estimate/app/api/routes.py
- FOUND: apps/efofx-estimate/tests/services/test_chat_service.py
- FOUND: .planning/phases/03-llm-integration/03-03-SUMMARY.md
- FOUND: commit fcadd22 (update chat models with ScopingContext)
- FOUND: commit 1da5098 (rewrite ChatService + tests)
