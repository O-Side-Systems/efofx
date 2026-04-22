# Story 1.1: Initialize White Label Widget Project

Status: review

## Story

As a developer,
I want a modern Vite + React + TypeScript widget project scaffolded with all architectural decisions implemented,
So that I can start building the embeddable chat widget on a solid foundation.

## Acceptance Criteria

**Given** the architecture specifies Vite + React 19 + TypeScript + Tailwind + Shadow DOM
**When** I run the widget initialization commands
**Then** the project structure is created at `apps/efofx-widget/` with:
- Vite configuration for single `embed.js` bundle output
- React 19 and TypeScript 5.x installed
- Tailwind CSS configured with PostCSS
- `vite-plugin-css-injected-by-js` for inline CSS
- Shadow DOM wrapper component stub
- Package.json with build scripts
- Basic folder structure: `src/components/`, `src/api/`, `src/hooks/`, `src/types/`

**And** running `npm run build` produces a single `dist/embed.js` file

**And** the widget can be embedded in a test HTML page with `<script src="dist/embed.js"></script>`

## Tasks / Subtasks

- [x] Create Vite project at `apps/efofx-widget/` with React + TypeScript template
- [x] Install and configure Tailwind CSS with PostCSS
- [x] Install `vite-plugin-css-injected-by-js` plugin
- [x] Configure Vite to output single `embed.js` bundle
- [x] Create basic folder structure (components, api, hooks, types)
- [x] Create Shadow DOM wrapper component stub
- [x] Verify build produces single bundle file
- [x] Test widget embedding in test HTML page

## Dev Notes

### Prerequisites

None (first story)

### Technical Notes

- Follow architecture doc: `docs/architecture.md` → Project Initialization → White Label Chat Widget
- Use exact commands from architecture (npm create vite, tailwind init)
- Verify output bundle size is reasonable (<600KB before optimization)
- Document widget initialization pattern in README.md

### References

- [Source: docs/epics.md#Story-1-1]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

- docs/stories/1-1-initialize-white-label-widget-project.context.xml

### Agent Model Used

claude-sonnet-4-5-20250929

### Debug Log References

<!-- To be filled by dev agent during implementation -->

### Completion Notes List

- ✅ Successfully created Vite + React 19 + TypeScript project at apps/efofx-widget/
- ✅ Configured Tailwind CSS v4 with @tailwindcss/postcss plugin (v4 requires separate PostCSS package)
- ✅ Installed and configured vite-plugin-css-injected-by-js for inline CSS bundling
- ✅ Configured Vite to output single IIFE bundle (embed.js) for widget embedding
- ✅ Created folder structure: src/components/, src/api/, src/hooks/, src/types/
- ✅ Implemented Shadow DOM wrapper component for style isolation
- ✅ Build successful: dist/embed.js = 578KB (under 600KB target)
- ✅ Created test-embed.html demonstrating widget can be embedded with <script> tag
- Widget exposes global efofxWidget.init() function for initialization
- Auto-initializes in development mode for easy local testing

### File List

**NEW:**
- apps/efofx-widget/ (entire project directory)
- apps/efofx-widget/package.json
- apps/efofx-widget/vite.config.ts
- apps/efofx-widget/tailwind.config.js
- apps/efofx-widget/postcss.config.js
- apps/efofx-widget/src/main.tsx (modified for widget initialization)
- apps/efofx-widget/src/index.css (modified for Tailwind directives)
- apps/efofx-widget/src/components/ShadowDOMWrapper.tsx
- apps/efofx-widget/src/api/ (directory)
- apps/efofx-widget/src/hooks/ (directory)
- apps/efofx-widget/src/types/ (directory)
- apps/efofx-widget/dist/embed.js (build output)
- apps/efofx-widget/test-embed.html

**MODIFIED:** None (greenfield component)
