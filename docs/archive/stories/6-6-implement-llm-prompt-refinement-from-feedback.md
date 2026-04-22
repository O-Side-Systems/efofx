# Story 6.6: Implement LLM Prompt Refinement from Feedback

Status: backlog

## Story

As a system,
I want to improve LLM narrative prompts based on feedback comments,
So that assumptions and risks become more accurate over time.

## Acceptance Criteria

**Given** feedback includes qualitative comments
**When** I implement prompt refinement
**Then** monthly review process:
1. Analyze feedback comments (extract common themes)
2. Generate prompt improvements (use LLM for suggestions)
3. Create prompt version (copy JSON, update, commit, A/B test)
4. Measure impact (compare v1.0.0 vs v1.1.0 after 30 days)

**And** `GET /admin/prompts/performance` shows prompt version performance comparison and recommendations

## Tasks / Subtasks

- [ ] Implement feedback comment analysis
- [ ] Extract common themes from comments
- [ ] Generate prompt improvement suggestions
- [ ] Create prompt version workflow
- [ ] Implement A/B testing (50/50 split)
- [ ] Create `GET /admin/prompts/performance` endpoint
- [ ] Measure impact after 30 days
- [ ] Promote better-performing version
- [ ] Test prompt refinement workflow

## Dev Notes

### Prerequisites

Story 4.2 (prompt versioning), Story 6.1 (feedback comments collected)

### Technical Notes

- Start with manual review (MVP), automate with NLP post-MVP
- Use LLM (GPT-4) to analyze feedback themes (meta!)
- Prompt improvements require code review (prompts are code)
- Track prompt version performance in calibration metrics
- Consider domain-specific prompts (construction vs IT/dev)

### References

- [Source: docs/epics.md#Story-6-6]
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
