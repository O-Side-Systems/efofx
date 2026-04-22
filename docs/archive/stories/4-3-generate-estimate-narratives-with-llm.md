# Story 4.3: Generate Estimate Narratives with LLM

Status: backlog

## Story

As a user,
I want my estimate explained in plain language with assumptions and risks highlighted,
So that I understand what the numbers mean and what might change them.

## Acceptance Criteria

**Given** a baseline estimate is calculated
**When** I request narrative generation
**Then** `app/services/llm_service.py::generate_estimate_narrative()`:
- Loads `estimate_narrative.json` prompt
- Renders prompt with: project_type, region, p50_cost, p80_cost, breakdown
- Calls OpenAI API with rendered prompt
- Returns narrative (150-300 words)
- Stores prompt_version in estimate document

**And** narrative includes:
- Explanation of P50 vs P80
- Key assumptions
- Risk factors
- Next steps

**And** when LLM call fails
**Then** graceful degradation: return estimate without narrative, log error to Sentry

**And** narrative generation adds <3 seconds to estimate API response time

## Tasks / Subtasks

- [ ] Implement `generate_estimate_narrative()` function
- [ ] Load and render narrative prompt
- [ ] Call OpenAI API
- [ ] Validate narrative length (150-300 words)
- [ ] Store prompt version in estimate
- [ ] Implement graceful degradation on error
- [ ] Test narrative quality
- [ ] Test response time <3 seconds

## Dev Notes

### Prerequisites

Story 4.2 (prompt loader ready)

### Technical Notes

- Use temperature=0.7 for some creativity while staying factual
- Max tokens=500 keeps narratives concise
- Consider streaming for instant user feedback (post-MVP)
- A/B test prompt versions for user satisfaction

### References

- [Source: docs/epics.md#Story-4-3]
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
