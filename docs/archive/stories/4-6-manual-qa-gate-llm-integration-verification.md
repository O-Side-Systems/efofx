# Story 4.6: Manual QA Gate - LLM Integration Verification

Status: backlog

## Story

As a QA tester,
I want a test guide to verify LLM integration produces helpful, accurate narratives,
So that I can approve Epic 4 before widget development begins.

## Acceptance Criteria

**Given** Epic 4 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-4-llm-integration-qa-guide.md` includes sections for:
- Prerequisites (test tenant with OpenAI key, sample project descriptions)
- Test Cases - BYOK (tenant key usage, platform fallback, error handling)
- Test Cases - Narrative Quality (content, length, tone)
- Test Cases - Conversational Scoping (chat flow, extraction, clarification)
- Test Cases - Prompt Versioning (version tracking, updates)
- Test Cases - Caching (hit rate, performance)
- Expected Results
- Known Limitations

## Tasks / Subtasks

- [ ] Create comprehensive QA test guide
- [ ] Document prerequisites
- [ ] Create BYOK test cases
- [ ] Create narrative quality test cases
- [ ] Create conversational scoping test cases
- [ ] Create prompt versioning test cases
- [ ] Create caching test cases
- [ ] Document expected results
- [ ] Document known limitations

## Dev Notes

### Prerequisites

Stories 4.1-4.5

### Technical Notes

- Review 10+ generated narratives for quality and consistency
- Test with edge cases: very large projects, unusual regions, custom features
- Verify OpenAI token usage matches expectations (~500 tokens per narrative)

### References

- [Source: docs/epics.md#Story-4-6]
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
