---
phase: 09-shared-library-extraction
plan: "01"
subsystem: workspace-infrastructure
tags: [python, typescript, uv, npm-workspaces, monorepo, shared-packages]
dependency_graph:
  requires: []
  provides:
    - "BOUNDARY.md extraction decision table"
    - "uv workspace config for Python monorepo"
    - "npm workspace config for TypeScript monorepo"
    - "packages/efofx-shared/ Python package skeleton"
    - "packages/efofx-ui/ TypeScript package skeleton"
  affects:
    - "09-02: Python extraction depends on efofx-shared skeleton"
    - "09-03: TypeScript extraction depends on efofx-ui skeleton"
    - "apps/efofx-estimate: will add efofx-shared as workspace dependency in 09-02"
    - "apps/efofx-widget, apps/efofx-dashboard: will add @efofx/ui as workspace dependency in 09-03"
tech_stack:
  added:
    - "uv.toml workspace configuration (Python monorepo)"
    - "root package.json with npm workspaces"
    - "packages/efofx-shared/ with setuptools build backend"
    - "packages/efofx-ui/ with TypeScript strict config"
  patterns:
    - "uv workspace: local packages referenced with { workspace = true } (no PyPI publishing)"
    - "npm workspace: @efofx/ui linked via workspace:* protocol"
    - "CSS modules planned for extracted components (camelCase class names)"
key_files:
  created:
    - ".planning/phases/09-shared-library-extraction/BOUNDARY.md"
    - "uv.toml"
    - "package.json"
    - "package-lock.json"
    - "packages/efofx-shared/pyproject.toml"
    - "packages/efofx-shared/efofx_shared/__init__.py"
    - "packages/efofx-shared/efofx_shared/core/__init__.py"
    - "packages/efofx-shared/efofx_shared/utils/__init__.py"
    - "packages/efofx-ui/package.json"
    - "packages/efofx-ui/tsconfig.json"
    - "packages/efofx-ui/src/index.ts"
  modified: []
decisions:
  - "uv not installed in dev environment — pyproject.toml and uv.toml validated via python3 tomllib; npm install used to verify frontend workspace (succeeded)"
  - "efofx-shared pyproject.toml has ZERO fastapi/motor/uvicorn dependencies — dependencies: pydantic>=2.11.0, cryptography>=41.0 only"
  - "efofx-ui uses source imports (main: ./src/index.ts) — no build step in shared package; consuming app's Vite resolves TypeScript directly"
  - "calculation_utils.py and validation_utils.py both STAY in apps/efofx-estimate — estimation-domain code despite generic names"
  - "_objectid.py STAYS — imports bson.ObjectId (pymongo dependency), cannot go in framework-agnostic shared package"
metrics:
  duration: "~8 min"
  completed_date: "2026-03-16"
  tasks_completed: 2
  files_created: 11
  files_modified: 0
---

# Phase 9 Plan 01: Boundary Document and Workspace Scaffolding Summary

**One-liner:** uv + npm workspace configs created with BOUNDARY.md decision table and skeleton packages for efofx-shared (Python) and @efofx/ui (TypeScript)

## What Was Built

This plan establishes the monorepo workspace infrastructure that all subsequent extraction plans depend on. Three deliverables:

1. **BOUNDARY.md** — module-by-module extraction decision table covering all 14 Python modules in `apps/efofx-estimate/app/` and all 17 TypeScript components across `apps/efofx-widget/` and `apps/efofx-dashboard/`. Every "stays" vs "extract" decision has a concrete rationale.

2. **Workspace configs** — `uv.toml` (Python workspace with apps/efofx-estimate + packages/efofx-shared members) and root `package.json` (npm workspace with apps/efofx-widget + apps/efofx-dashboard + packages/efofx-ui). `npm install` succeeded and `@efofx/ui` is symlinked in `node_modules/@efofx/ui`.

3. **Package skeletons** — `packages/efofx-shared/` with clean `pyproject.toml` (setuptools, pydantic + cryptography only — zero app-server deps) and `packages/efofx-ui/` with `package.json` (react as peerDependency, TypeScript strict config matching widget settings).

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Boundary document and workspace configs | e6a5f5a | BOUNDARY.md, uv.toml, package.json |
| 2 | Scaffold shared package skeletons | 94d2f83 | packages/efofx-shared/pyproject.toml, packages/efofx-ui/package.json, tsconfig.json, src/index.ts |

## Decisions Made

- **uv not installed** — The project setup docs (SETUP.md) use standard Python venv, not uv. Config files validated via Python's `tomllib` stdlib module. uv will need to be installed before 09-02 can run `uv sync`. This is not a blocker for this plan — the file artifacts are correct.
- **calculation_utils.py stays** — Pool construction domain logic (region multipliers, labor cost calculations). File name suggests generic utility but content is estimation-specific — moving it would mislead future IT/dev vertical developers.
- **_objectid.py stays** — Imports `bson.ObjectId` (pymongo ecosystem). The shared package must have no MongoDB dependencies.
- **Source imports for @efofx/ui** — `main` points to `./src/index.ts` (TypeScript source). No Vite library build step needed in the shared package — consuming apps' Vite handles TypeScript resolution.

## Deviations from Plan

### Auto-noted Issues

**1. [Rule 3 - Environment] uv not available in shell PATH**
- **Found during:** Task 2 — `uv sync` command
- **Issue:** uv binary not found via `which uv` or common paths; project setup docs use standard Python venv, not uv
- **Fix:** Validated TOML configs via `python3 tomllib` (both valid). Verified npm workspace via `npm install` (succeeded). Python workspace will require uv installation before 09-02 extraction.
- **Impact:** This plan's artifacts are correct and complete. `uv sync` is a future concern for 09-02.
- **Deferred:** Document for 09-02 executor — install uv before running extraction tasks

## Self-Check: PASSED

All files verified on disk. Both commits verified in git history.

| Item | Status |
|------|--------|
| BOUNDARY.md | FOUND |
| uv.toml | FOUND |
| package.json | FOUND |
| packages/efofx-shared/pyproject.toml | FOUND |
| packages/efofx-shared/efofx_shared/__init__.py | FOUND |
| packages/efofx-ui/package.json | FOUND |
| packages/efofx-ui/src/index.ts | FOUND |
| Commit e6a5f5a | FOUND |
| Commit 94d2f83 | FOUND |
