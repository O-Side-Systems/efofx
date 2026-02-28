# Phase 3: LLM Integration - Context

**Gathered:** 2026-02-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Real AI-powered conversational estimation: users describe their project through a structured chat, the system asks targeted follow-ups, and generates a human-readable estimate narrative with P50/P80 ranges streamed via SSE. Includes per-request BYOK key injection, prompt versioning, response caching, and graceful error handling. The widget UI (Phase 4) consumes these APIs — this phase builds the backend intelligence layer.

</domain>

<decisions>
## Implementation Decisions

### Conversation flow
- Structured guided intake — system leads with a logical question sequence, not freeform
- 3-5 follow-up questions: project type, size, location, timeline, special conditions
- Quick path to estimate: optimize for speed over exhaustive detail gathering
- Auto-trigger with confirmation — when system has enough detail, it says "I have enough to generate an estimate. Ready?" and user confirms before generation starts
- User can also explicitly request estimate generation at any point

### Estimate narrative style
- Plain language, no jargon — "You'll most likely spend between $45k-$52k, though it could reach $65k if conditions change"
- Summary first, then breakdown — lead with bottom-line P50/P80 total range, then stream detailed cost categories and adjustment factors
- The narrative should explain the reasoning behind ranges in terms a homeowner understands
- Each cost category gets a plain-English explanation of what it covers and why the range exists

### Error experience
- Transparent with retry — "We're having trouble generating a response. Retrying..." then "Please try again in a moment" if retry fails
- Show what's happening, don't hide failures behind vague messages
- Conversation is preserved across errors — user doesn't lose their progress
- Differentiate between transient failures (retry) and permanent failures (key exhausted → 402 with clear message to contractor)

### Prompt versioning
- System-managed only — prompts are versioned JSON files in git, controlled by the platform team
- Contractors cannot edit or customize prompts
- Each estimate records which prompt_version was used for traceability
- Prompt versions are immutable once published — new version for any change

### Claude's Discretion
- Exact prompt content and structure (system prompts, follow-up generation prompts, narrative generation prompts)
- Cache key strategy for response deduplication
- SSE event format and reconnection behavior
- Chat session TTL duration
- Specific error retry timing and backoff strategy

</decisions>

<specifics>
## Specific Ideas

- The conversation should feel like talking to a knowledgeable contractor, not filling out a form — even though the questions are structured
- P50/P80 explanation: "most likely" for P50, "could reach" or "budget for" for P80 — avoid statistical terminology
- The estimate narrative is the product's key differentiator — it should feel thoughtful and specific to the project, not generic boilerplate
- Streaming should show a "thinking" state before tokens start arriving — the user should know something is happening

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-llm-integration*
*Context gathered: 2026-02-27*
