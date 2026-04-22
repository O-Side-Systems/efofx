# Story 4.2: Implement Git-Based Prompt Management

Status: backlog

## Story

As a developer,
I want prompts stored as version-controlled JSON files,
So that prompt changes are tracked, reviewable, and each estimate links to its prompt version.

## Acceptance Criteria

**Given** the architecture specifies git-based JSON prompts
**When** I create prompt files
**Then** `config/prompts/` contains:
- `estimate_narrative.json` - P50/P80 explanation prompt
- `conversational_scoping.json` - Chat bot prompt
- `feedback_analysis.json` - Calibration feedback prompt (future)

**And** each prompt file structure includes version, created_at, model, system_prompt, user_template, temperature, max_tokens

**And** `app/utils/prompt_loader.py` contains:
- `PromptLoader` class that reads JSON files
- In-memory cache of loaded prompts
- `render(name, **kwargs)` that formats user_template with variables

**And** when generating estimate
**Then** prompt version stored in estimate document: `{"prompt_version": "1.0.0"}`

**And** when prompt file changes
**Then** git commit tracks the change with reason

## Tasks / Subtasks

- [ ] Create `config/prompts/` directory
- [ ] Create `estimate_narrative.json` prompt file
- [ ] Create `conversational_scoping.json` prompt file
- [ ] Create `feedback_analysis.json` prompt file
- [ ] Implement PromptLoader class
- [ ] Add in-memory caching
- [ ] Implement template rendering
- [ ] Store prompt version with estimates
- [ ] Test prompt loading and rendering

## Dev Notes

### Prerequisites

Story 4.1 (LLM client ready)

### Technical Notes

- Prompts are code - require PR review before merge
- Use semantic versioning for prompts (1.0.0, 1.1.0, 2.0.0)
- Store prompt_version with every estimate for calibration analysis
- Post-MVP: Migrate to LangSmith for A/B testing and analytics

### References

- [Source: docs/epics.md#Story-4-2]
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
