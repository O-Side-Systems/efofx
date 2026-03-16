---
phase: 09-shared-library-extraction
plan: "02"
subsystem: shared-library
tags: [python, uv, workspace, shared-package, crypto, enums, extraction, isolation-test]

dependency_graph:
  requires:
    - phase: "09-01"
      provides: "packages/efofx-shared/ skeleton, uv.toml workspace config"
  provides:
    - "packages/efofx-shared/efofx_shared/utils/crypto.py — Fernet HKDF encryption utilities"
    - "packages/efofx-shared/efofx_shared/core/constants.py — 4 pure enums (EstimationStatus, ReferenceClassCategory, CostBreakdownCategory, Region)"
    - "packages/efofx-shared/tests/test_isolation.py — venv isolation test (no fastapi/motor/uvicorn)"
    - "apps/efofx-estimate/app/utils/crypto.py — re-export shim (backward compatible)"
    - "apps/efofx-estimate/app/core/constants.py — re-exports enums from efofx_shared, keeps app-specific constants"
    - "workspace dependency link: efofx-shared installed editable in apps/efofx-estimate venv"
  affects:
    - "09-03: TypeScript extraction (unrelated, separate package)"
    - "09-04: Final verification across both shared packages"

tech-stack:
  added:
    - "uv pip install -e (editable install of local workspace package)"
    - "root pyproject.toml [tool.uv.workspace] for Python monorepo workspace"
    - "[tool.uv.sources] efofx-shared = { workspace = true } in apps/efofx-estimate"
  patterns:
    - "Re-export shim: app/utils/crypto.py does 'from efofx_shared.utils.crypto import *' — backward-compat preserved without touching callers"
    - "Re-export + keep: app/core/constants.py imports 4 enums from shared, keeps API_MESSAGES etc in place"
    - "Isolation test: venv.create() + subprocess install + subprocess import check for prohibited packages"

key-files:
  created:
    - "packages/efofx-shared/efofx_shared/utils/crypto.py"
    - "packages/efofx-shared/efofx_shared/core/constants.py"
    - "packages/efofx-shared/tests/__init__.py"
    - "packages/efofx-shared/tests/test_isolation.py"
    - "pyproject.toml (root workspace config)"
  modified:
    - "packages/efofx-shared/efofx_shared/__init__.py"
    - "packages/efofx-shared/efofx_shared/utils/__init__.py"
    - "packages/efofx-shared/efofx_shared/core/__init__.py"
    - "packages/efofx-shared/pyproject.toml"
    - "apps/efofx-estimate/app/utils/crypto.py"
    - "apps/efofx-estimate/app/core/constants.py"
    - "apps/efofx-estimate/pyproject.toml"
    - "uv.toml"

key-decisions:
  - "uv.toml workspace config is invalid — uv requires [tool.uv.workspace] in pyproject.toml not [workspace] in uv.toml; fixed by creating root pyproject.toml"
  - "uv sync blocked by private Azure DevOps pip index requiring interactive auth; used 'uv pip install -e' targeting the existing .venv directly (bypasses index auth)"
  - "Re-export pattern chosen over direct import updates: app/utils/crypto.py becomes a shim, no caller changes needed across entire codebase"
  - "Integration test suite (377 tests) requires live MongoDB Atlas — hangs in dev environment; unit tests (test_crypto.py 9/9, test_reference_class_model.py 8/8) pass and confirm no regressions from import changes"
  - "Isolation test uses uv pip when available (fast, no private-index prompt) with stdlib venv fallback"

patterns-established:
  - "Re-export shim: 'from efofx_shared.utils.crypto import *  # noqa: F401,F403' — one-line backward-compat shim"
  - "Selective re-export: import specific symbols from shared package at top of app module, keep app-specific constants below"

requirements-completed:
  - EXTR-02
  - EXTR-04

duration: ~20min
completed: "2026-03-15"
---

# Phase 9 Plan 02: Python Extraction (crypto + enums) Summary

**Fernet HKDF crypto utilities and 4 pure enums extracted into efofx-shared with venv isolation test confirming zero app-server dependency leaks**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-03-15T00:00:00Z
- **Completed:** 2026-03-15
- **Tasks:** 2
- **Files modified:** 12 files (6 created, 6 modified)

## Accomplishments

- `packages/efofx-shared/efofx_shared/utils/crypto.py` — full crypto module with HKDF key derivation, encrypt/decrypt/mask functions (no app.* imports)
- `packages/efofx-shared/efofx_shared/core/constants.py` — 4 pure enums (EstimationStatus, ReferenceClassCategory, CostBreakdownCategory, Region) only
- Re-export shims in app preserve backward compatibility — zero callers required changes
- Isolation test confirms: `import efofx_shared` succeeds in fresh venv; `import fastapi/motor/uvicorn` all fail (returncode != 0)
- 9/9 crypto unit tests pass; 8/8 model unit tests pass after import changes

## Task Commits

Each task was committed atomically:

1. **Task 1: Extract crypto.py and constants enums to shared package** - `882eb31` (feat)
2. **Task 2: Create isolation test and verify existing tests pass** - `122f9af` (feat)

## Files Created/Modified

- `packages/efofx-shared/efofx_shared/utils/crypto.py` — HKDF Fernet crypto functions (copied verbatim, no app.* imports)
- `packages/efofx-shared/efofx_shared/core/constants.py` — 4 pure str enums for shared domain types
- `packages/efofx-shared/efofx_shared/__init__.py` — updated to export version + all public symbols
- `packages/efofx-shared/efofx_shared/utils/__init__.py` — updated to re-export crypto functions
- `packages/efofx-shared/efofx_shared/core/__init__.py` — updated to re-export enum classes
- `packages/efofx-shared/pyproject.toml` — added dev deps (pytest) and pytest config
- `packages/efofx-shared/tests/__init__.py` — empty package marker
- `packages/efofx-shared/tests/test_isolation.py` — venv isolation test with subprocess import checks
- `apps/efofx-estimate/app/utils/crypto.py` — replaced with 2-line re-export shim
- `apps/efofx-estimate/app/core/constants.py` — imports 4 enums from efofx_shared, keeps app constants
- `apps/efofx-estimate/pyproject.toml` — added efofx-shared dep + [tool.uv.sources] workspace declaration
- `pyproject.toml` (new, root) — [tool.uv.workspace] members declaration (required by uv)
- `uv.toml` — cleared workspace config (moved to pyproject.toml)

## Decisions Made

- **uv.toml workspace config is invalid**: uv 0.10.x requires workspace declaration in `[tool.uv.workspace]` inside `pyproject.toml`, not in `uv.toml`. Created root `pyproject.toml` and cleared `uv.toml`.
- **Private Azure DevOps index blocks uv sync**: The global pip config has an Azure DevOps extra-index-url that prompts for credentials interactively. Used `uv pip install -e` targeting the existing `.venv` directly, which resolves from local disk only (no network for the efofx-shared package).
- **Re-export shim pattern**: One-line `from efofx_shared.utils.crypto import *` in the app module means zero caller changes needed anywhere in the codebase.
- **MongoDB integration test suite not run**: 377 tests require a live MongoDB Atlas connection and hang indefinitely in the dev environment. This is pre-existing behavior. Unit test subsets (crypto 9/9, model 8/8, services 68/69 with 1 pre-existing performance failure) confirm import changes introduced no regressions.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] uv.toml workspace config invalid in uv 0.10.x**
- **Found during:** Task 1 — `uv sync` command
- **Issue:** `[workspace]` in `uv.toml` is not supported; uv requires `[tool.uv.workspace]` in a `pyproject.toml`
- **Fix:** Created root `pyproject.toml` with `[tool.uv.workspace]`, cleared `uv.toml`
- **Files modified:** `pyproject.toml` (created), `uv.toml` (cleared)
- **Verification:** `uv sync` error resolved; `uv pip install` succeeded
- **Committed in:** 882eb31 (Task 1 commit)

**2. [Rule 3 - Blocking] [tool.uv.sources] missing from apps/efofx-estimate/pyproject.toml**
- **Found during:** Task 1 — second `uv sync` attempt
- **Issue:** uv requires `[tool.uv.sources] efofx-shared = { workspace = true }` alongside the dependency declaration
- **Fix:** Added `[tool.uv.sources]` section to `apps/efofx-estimate/pyproject.toml`
- **Files modified:** `apps/efofx-estimate/pyproject.toml`
- **Verification:** uv pip install resolved workspace package correctly
- **Committed in:** 882eb31 (Task 1 commit)

**3. [Rule 3 - Blocking] Azure DevOps private index blocks `uv sync`**
- **Found during:** Task 1 — third `uv sync` attempt
- **Issue:** Global pip config has an extra-index-url pointing to Azure DevOps that requires interactive credential prompt; `uv sync` times out trying to download all packages
- **Fix:** Used `uv pip install -e` targeting the existing `.venv/bin/python` directly; resolves efofx-shared from disk only (no network needed for local package)
- **Files modified:** None (installation, not file change)
- **Verification:** `uv pip install` completed in 2s; import verified successfully
- **Committed in:** N/A (environment action, not file change)

---

**Total deviations:** 3 auto-fixed (3 blocking environment/config issues)
**Impact on plan:** All auto-fixes were uv configuration corrections. The workspace link works correctly. No scope creep.

## Issues Encountered

- Git stash accidentally applied during test verification (checking pre-existing performance test failure), creating a merge conflict in `widget.py`. Resolved by accepting the upstream version (the correct committed state). No functional impact.

## Next Phase Readiness

- `efofx_shared.utils.crypto` and `efofx_shared.core.constants` are ready for consumption by any service
- Isolation test enforces the dependency boundary in CI
- 09-03 (TypeScript UI component extraction) can proceed independently — Python extraction complete
- 09-04 (final verification) can include the Python workspace link as part of end-to-end verification

## Self-Check: PASSED

| Item | Status |
|------|--------|
| packages/efofx-shared/efofx_shared/utils/crypto.py | FOUND |
| packages/efofx-shared/efofx_shared/core/constants.py | FOUND |
| packages/efofx-shared/tests/test_isolation.py | FOUND |
| pyproject.toml (root workspace) | FOUND |
| 09-02-SUMMARY.md | FOUND |
| Commit 882eb31 | FOUND |
| Commit 122f9af | FOUND |
