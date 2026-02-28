# Phase 4: White-Label Widget - Context

**Gathered:** 2026-02-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Embeddable JavaScript widget that contractors paste onto their websites via a single script tag. Site visitors interact with a fully branded chat experience to describe their project, submit contact info, and receive a probabilistic estimate — all without leaving the host page. No "Powered by efOfX" branding visible.

</domain>

<decisions>
## Implementation Decisions

### Widget trigger & placement
- Support two embed modes: floating button (default) and inline embed (via target container ID in script config)
- Floating button: fixed position, expands into slide-up panel on desktop (~500px height with internal scroll)
- Mobile: full-screen takeover when widget opens, X button or back to close
- Inline embed: widget renders inside a contractor-placed container div on the page

### Chat experience
- Bubble-style messages: user messages right-aligned in brand primary color, system messages left-aligned in neutral
- Animated three-dot typing indicator (iMessage-style) while system generates responses
- Widget header: contractor logo on left, company name, X button to minimize/close
- First message is the contractor's custom welcome text from branding config (BRND-03)

### Lead capture flow
- Lead capture form gates the estimate — appears after conversation is complete, before estimate is shown
- Required fields: name, email, phone (all three required)
- After form submission, returns to chat view with animated dots showing estimate is being generated
- Estimate generation happens server-side while the thinking indicator displays

### Estimate presentation
- P50/P80 ranges shown as horizontal range bar visualization with dollar amounts labeled ("Most likely: $X — High end: $Y")
- Cost breakdown categories displayed as expandable accordion rows (category name + subtotal, expands to line items)
- LLM-generated narrative streams into chat as a message below the estimate card (uses existing SSE streaming from Phase 3)
- Prominent disclaimer below estimate card, above narrative: estimates are unofficial ballpark figures based on similar projects, no figure is binding, official requirements need human consultation
- "Request Free Consultation" CTA button accompanies the disclaimer — drives visitors toward real contractor engagement

### Claude's Discretion
- Floating button appearance (circle with icon vs text pill, animation style)
- Exact spacing, typography, and visual polish within the widget
- Error state handling and retry UX
- Transition animations between widget states (collapsed → expanded → form → estimate)
- Input field placeholder text and microcopy

</decisions>

<specifics>
## Specific Ideas

- Disclaimer must be prominent and drive users toward requesting a consultation — the estimate is a lead generation tool, not a binding quote
- The "Request Free Consultation" CTA is a key conversion point — it should feel like a natural next step after seeing the estimate
- Chat bubbles should feel lightweight and fast, not heavy or enterprise-y
- The full-screen mobile takeover should feel native, not like a trapped web view

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 04-white-label-widget*
*Context gathered: 2026-02-27*
