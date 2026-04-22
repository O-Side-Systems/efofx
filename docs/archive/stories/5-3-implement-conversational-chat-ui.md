# Story 5.3: Implement Conversational Chat UI

Status: backlog

## Story

As a website visitor,
I want to describe my project in natural language through chat,
So that I get an estimate without filling out boring forms.

## Acceptance Criteria

**Given** FR-6.3 specifies conversational UI
**When** I implement the chat interface
**Then** `apps/efofx-widget/src/components/Chat.tsx` contains:
- Message list with auto-scroll to bottom
- User message bubbles (right-aligned, primary color)
- Bot message bubbles (left-aligned, gray)
- Typing indicator animation while bot is responding
- Quick-select suggestion buttons
- Input field with send button
- Mobile-optimized keyboard handling

**And** chat flow works end-to-end (welcome → user message → bot response → loop until ready_for_estimate)

**And** message history persists in session (survives page refresh for 24 hours)

**And** response time: bot messages appear within 2 seconds

## Tasks / Subtasks

- [ ] Create Chat.tsx component
- [ ] Implement message list with auto-scroll
- [ ] Create user and bot message bubble components
- [ ] Add typing indicator animation
- [ ] Create suggestion button components
- [ ] Create input field with send functionality
- [ ] Optimize for mobile keyboards
- [ ] Store chat history in sessionStorage
- [ ] Test full conversation flow
- [ ] Test response time

## Dev Notes

### Prerequisites

Story 4.4 (chat API ready), Story 5.2 (branding applied)

### Technical Notes

- Use React 19 `useTransition` for optimistic UI updates
- Store chat history in sessionStorage (session_token key)
- Implement message retry on network failure
- Accessibility: ARIA labels, keyboard navigation
- Mobile: Input field fixed at bottom, prevents iOS keyboard jump

### References

- [Source: docs/epics.md#Story-5-3]
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
