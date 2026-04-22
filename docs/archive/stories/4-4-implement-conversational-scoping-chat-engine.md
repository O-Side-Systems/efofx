# Story 4.4: Implement Conversational Scoping Chat Engine

Status: backlog

## Story

As a user,
I want to describe my project in natural language through chat,
So that the system gathers details without forcing me to fill out forms.

## Acceptance Criteria

**Given** the widget needs conversational project scoping
**When** I implement chat engine
**Then** `app/services/chat_service.py` contains:
- `ChatSession` class that maintains conversation state
- `send_message(session_id, user_message, tenant_id)` function
- Loads `conversational_scoping.json` prompt
- Uses OpenAI function calling to extract structured data
- Returns bot response + extracted attributes
- Determines when enough info exists for estimate (confidence >= 0.7)

**And** `POST /api/v1/chat/send` endpoint accepts: session_id, message, tenant_id (from JWT)
**And** Returns: bot_message, extracted_attributes, ready_for_estimate

**And** conversation history stored in MongoDB

**And** when insufficient info
**Then** bot asks clarifying questions based on missing attributes

## Tasks / Subtasks

- [ ] Create `app/services/chat_service.py`
- [ ] Implement ChatSession class
- [ ] Implement `send_message()` function
- [ ] Load conversational scoping prompt
- [ ] Implement OpenAI function calling for extraction
- [ ] Create `POST /api/v1/chat/send` endpoint
- [ ] Store conversation history in MongoDB
- [ ] Implement missing attributes detection
- [ ] Test full conversation flow

## Dev Notes

### Prerequisites

Story 4.2 (prompt loader ready)

### Technical Notes

- Use OpenAI structured outputs (JSON mode) for attribute extraction
- Session expires after 24 hours of inactivity
- Store chat history: {session_id, tenant_id, messages[], extracted_attributes}
- Widget calls this API for each user message

### References

- [Source: docs/epics.md#Story-4-4]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

<!-- To be filled by dev agent -->

### Debug Log References

<!-- To be filled by dev agent during implementation -->

### Completion Notes List

<!-- To be filled by dev agent upon completion -->

### File List

<!-- NEW/MODIFIED/DELETED files will be listed here by dev agent -->
