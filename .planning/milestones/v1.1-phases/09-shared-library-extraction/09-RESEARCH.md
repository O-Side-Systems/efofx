# Phase 9: Shared Library Extraction - Research

**Researched:** 2026-03-15
**Domain:** Python monorepo packaging (uv workspaces), npm workspaces, CI isolation testing, code quality documentation
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Package boundaries
- Extract ALL reusable utilities to packages/efofx-shared/, not just multi-vertical ones — anything not estimation-specific moves
- Mirror the current app structure: packages/efofx-shared/efofx_shared/core/, utils/, models/ — familiar layout
- Immediate re-import: move code to shared package and update app imports in the same phase — one clean cut, no deprecation period
- Boundary document format: decision table (Module | Location | Rationale) covering every module with clear reasoning
- Use uv workspaces for Python monorepo linking — no PyPI publishing, changes immediately available

#### Component extraction scope
- Audit ALL components across efofx-widget and efofx-dashboard — extract everything reusable, not just the 3 named components
- The named 3 (EstimateCard, ChatBubble, TypingIndicator) are the minimum; additional generic components should also move
- CSS modules co-located with each component (.module.css files) — scoped by default, overridable by consuming apps
- Use npm workspaces for frontend monorepo linking — no npm publishing, workspace protocol in package.json

#### Code quality standards
- Comprehensive STANDARDS.md at repo root (alongside CLAUDE.md) covering: code style, file/folder structure, testing patterns, documentation (docstrings, README), error handling patterns, logging conventions
- Audit the ENTIRE codebase for conformance — not just the extracted packages
- Fix ALL violations found during the audit in this phase — leave the codebase fully conformant, not just catalogued

#### CI isolation strategy
- CI checks BOTH packages: Python import isolation AND TypeScript build isolation
- Platform: DigitalOcean App Platform (already used for hosting — keep everything in one place)
- Create CI from scratch — no existing CI workflows to extend
- Full test suite on every PR: isolation tests + pytest + npm build — catches regressions from extraction refactoring

### Claude's Discretion
- Exact modules to extract (determined by codebase audit)
- uv workspace configuration details
- npm workspace configuration details
- CI workflow file structure and job organization
- Order of extraction operations

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EXTR-01 | Shared library boundary document — what goes in shared vs stays in apps | Codebase audit findings below identify exact module placement; decision table format documented |
| EXTR-02 | Extract packages/efofx-shared/ Python package (crypto, validation, calculation utils) with uv workspace | uv workspace config patterns, package structure, import path updates documented |
| EXTR-03 | Extract packages/efofx-ui/ React components (EstimateCard, ChatBubble, TypingIndicator) with npm workspaces | npm workspace config, TypeScript package structure, CSS module co-location patterns documented |
| EXTR-04 | CI test verifying shared packages install in fresh env with zero app imports | DigitalOcean App Platform CI patterns, GitHub Actions isolation test strategy documented |
| EXTR-05 | Code quality standards documentation | Existing tooling (ruff, mypy, black, eslint, tsc) documented; conformance audit scope identified |
</phase_requirements>

## Summary

This phase extracts shared code from `apps/efofx-estimate/` into `packages/efofx-shared/` (Python) and from `apps/efofx-widget/` into `packages/efofx-ui/` (React/TypeScript). The primary technical challenge is not the extraction itself — it is ensuring the extracted packages have clean dependency boundaries (no FastAPI, Motor, or app-specific imports leaking in) and that a CI test enforces this automatically.

The Python codebase (`apps/efofx-estimate/`) is the authoritative app (not `apps/estimator-project/`, which appears to be an older prototype). The app uses `setuptools` build backend and Python 3.11+. The utilities that clearly belong in `efofx-shared` are `app/utils/crypto.py` (Fernet HKDF encryption — no FastAPI dependency) and the future home of shared validation/calculation utilities that have clean stdlib-only or `pydantic`-only dependency graphs. The `app/core/constants.py` enums and `app/models/_objectid.py` are candidates that need boundary analysis.

The frontend extraction involves moving `ChatBubble`, `EstimateCard`, and `TypingIndicator` from `apps/efofx-widget/src/components/` to `packages/efofx-ui/`. These three components have zero widget-specific state or estimation-domain logic — they are safe to extract immediately. The `LoadingSkeleton` component in `efofx-dashboard` is an additional generic candidate. npm workspace linking via the `workspace:` protocol means no publishing is required.

**Primary recommendation:** Work in four sequential waves — (1) create workspace scaffolding and Python package with uv, (2) Python extraction + re-import + tests, (3) TypeScript package scaffolding + component extraction + re-import, (4) CI workflow + boundary document + STANDARDS.md + conformance audit.

## Standard Stack

### Core (Python)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| uv | latest | Python workspace + packaging tool | Zero-config workspace linking; project already uses Python 3.11+; pip-compatible |
| setuptools | >=61 | Build backend for efofx-shared | Already used by efofx-estimate; no change in mental model |
| hatchling | — | Alternative build backend | Used in estimator-project (older prototype); NOT used by primary app — stick with setuptools |
| cryptography | already installed | Fernet HKDF — core of crypto.py | Only dependency of crypto.py; no FastAPI/Motor in the dep graph |
| pydantic | 2.11.x | Models that belong in shared | Already in efofx-estimate; shared models must NOT import from app modules |

### Core (TypeScript / npm)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| npm workspaces | npm 8+ | Monorepo linking | Already in Node.js; workspace: protocol = no publishing needed |
| TypeScript | ~5.9.3 | Shared package types | Matches current widget and dashboard tsconfig |
| vite | ^7.x | Build tooling | Used by both apps; shared UI package needs same build chain |
| @vitejs/plugin-react | ^5.x | React JSX transform | Required for React 19 JSX compilation |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| pytest | 8.4.1 | Unit tests for shared Python package | Isolation test: attempt import in subprocess, verify no app imports |
| github-actions | N/A | CI orchestration on DigitalOcean | DigitalOcean App Platform uses GitHub as source; CI runs in GitHub Actions |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| npm workspaces | pnpm workspaces | pnpm is faster but not installed; npm is already present, workspace: protocol works the same |
| setuptools | hatchling | hatchling is in estimator-project (older prototype); efofx-estimate uses setuptools — consistency with the live app wins |
| subprocess isolation test | tox environments | tox is simpler for isolation matrix but adds a CI dependency; subprocess venv creation in a pytest test is sufficient and already understood by the team |

**Installation (workspace root):**

```bash
# Python workspace — run from workspace root
uv init --workspace  # creates uv.toml with [workspace] section

# Frontend workspace root package.json
{
  "workspaces": ["apps/efofx-widget", "apps/efofx-dashboard", "packages/efofx-ui"]
}
```

## Architecture Patterns

### Recommended Workspace Structure

```
efofx-workspace/              # workspace root
├── package.json              # npm workspaces declaration (NEW)
├── uv.toml                   # uv workspace config (NEW)
├── STANDARDS.md              # code quality standards (NEW)
├── apps/
│   ├── efofx-estimate/       # Python backend (updated imports)
│   ├── efofx-widget/         # React widget (updated imports)
│   └── efofx-dashboard/      # React dashboard (updated imports)
└── packages/
    ├── efofx-shared/         # Python shared package (NEW)
    │   ├── pyproject.toml
    │   └── efofx_shared/
    │       ├── __init__.py
    │       ├── core/         # constants, enums (if pure stdlib/pydantic)
    │       ├── utils/        # crypto.py, calculation_utils.py
    │       └── models/       # shared pydantic models (no app/ imports)
    └── efofx-ui/             # TypeScript shared components (NEW)
        ├── package.json
        ├── tsconfig.json
        ├── vite.config.ts
        └── src/
            ├── index.ts      # barrel export
            └── components/
                ├── ChatBubble/
                │   ├── ChatBubble.tsx
                │   └── ChatBubble.module.css
                ├── EstimateCard/
                │   ├── EstimateCard.tsx
                │   └── EstimateCard.module.css
                ├── TypingIndicator/
                │   ├── TypingIndicator.tsx
                │   └── TypingIndicator.module.css
                └── LoadingSkeleton/ (if generic enough — see audit)
```

### Pattern 1: uv Workspace Configuration

**What:** `uv.toml` at workspace root declares members. Each member keeps its own `pyproject.toml`. Local packages are referenced with `{ workspace = true }`.

**When to use:** When Python packages in the monorepo need to share code without PyPI publishing.

```toml
# uv.toml (workspace root)
[workspace]
members = [
    "apps/efofx-estimate",
    "packages/efofx-shared",
]
```

```toml
# packages/efofx-shared/pyproject.toml
[build-system]
requires = ["setuptools>=61", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "efofx-shared"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "pydantic>=2.11.0",
    "cryptography>=41.0",  # for Fernet/HKDF in crypto.py
]
# NOTE: NO fastapi, NO motor, NO uvicorn — shared package must be framework-agnostic
```

```toml
# apps/efofx-estimate/pyproject.toml — add to dependencies
dependencies = [
    ...
    "efofx-shared @ { workspace = true }",
]
```

**Confidence:** MEDIUM — uv workspace syntax verified against uv documentation patterns. Exact `@ { workspace = true }` syntax should be confirmed in uv docs before writing.

### Pattern 2: npm Workspace Configuration

**What:** Root `package.json` declares `workspaces` array. Each package uses `"@efofx/ui": "workspace:*"` in its dependencies.

**When to use:** Sharing TypeScript/React components between multiple frontend apps without publishing to npm registry.

```json
// package.json (workspace root)
{
  "name": "efofx-workspace",
  "private": true,
  "workspaces": [
    "apps/efofx-widget",
    "apps/efofx-dashboard",
    "packages/efofx-ui"
  ]
}
```

```json
// packages/efofx-ui/package.json
{
  "name": "@efofx/ui",
  "version": "0.0.0",
  "private": true,
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "peerDependencies": {
    "react": ">=19.0.0",
    "react-dom": ">=19.0.0"
  }
}
```

```json
// apps/efofx-widget/package.json — add dependency
{
  "dependencies": {
    "@efofx/ui": "workspace:*"
  }
}
```

```tsx
// apps/efofx-widget/src/components/ChatPanel.tsx — updated import
import { ChatBubble, TypingIndicator } from '@efofx/ui';
```

**Confidence:** HIGH — npm workspaces with `workspace:*` protocol is standard practice verified by npm documentation.

### Pattern 3: CSS Modules in Shared Package

**What:** CSS modules (`.module.css`) co-located with each extracted component. Consuming apps can override via CSS custom properties — the current components already use `var(--brand-accent)`, `var(--brand-primary)`, `var(--brand-secondary)`.

**Why it matters:** The current components use inline CSS class names like `efofx-bubble-wrapper` defined in `widget.css`. After extraction, each component gets its own `.module.css` where the local classnames replace the global widget.css classnames. CSS custom properties remain the override hook for theming.

```css
/* packages/efofx-ui/src/components/ChatBubble/ChatBubble.module.css */
.bubbleWrapper { /* was: efofx-bubble-wrapper */ }
.bubbleWrapperUser { /* was: efofx-bubble-wrapper--user */ }
.bubbleWrapperAssistant { /* was: efofx-bubble-wrapper--assistant */ }
.bubble { /* was: efofx-bubble */ }
.bubbleUser { background: var(--brand-primary, #2563eb); }
.bubbleAssistant { background: var(--brand-secondary, #f1f5f9); }
```

**Confidence:** HIGH — pattern is standard; CSS custom property hooks are already in place.

### Pattern 4: Python Import Isolation Test

**What:** A pytest test that creates a fresh subprocess `venv`, installs only `efofx-shared`, then attempts `python -c "import efofx_shared"`. The test fails if the import raises an error (missing dep) or if the install pulled in `fastapi`, `motor`, or `uvicorn`.

```python
# packages/efofx-shared/tests/test_isolation.py
import subprocess
import sys
import venv
import tempfile
from pathlib import Path

def test_shared_package_imports_without_app_deps(tmp_path):
    """efofx-shared must install cleanly without FastAPI/Motor."""
    # Create fresh venv
    venv.create(str(tmp_path / "venv"), with_pip=True)
    pip = tmp_path / "venv" / "bin" / "pip"
    python = tmp_path / "venv" / "bin" / "python"

    # Install only the shared package
    project_root = Path(__file__).parent.parent
    result = subprocess.run(
        [str(pip), "install", str(project_root), "--quiet"],
        capture_output=True, text=True
    )
    assert result.returncode == 0, f"Install failed: {result.stderr}"

    # Verify import succeeds
    result = subprocess.run(
        [str(python), "-c", "import efofx_shared; print('ok')"],
        capture_output=True, text=True
    )
    assert result.returncode == 0, f"Import failed: {result.stderr}"

    # Verify prohibited packages are NOT installed
    for pkg in ["fastapi", "motor", "uvicorn"]:
        result = subprocess.run(
            [str(python), "-c", f"import {pkg}"],
            capture_output=True, text=True
        )
        assert result.returncode != 0, f"Prohibited package {pkg} is importable!"
```

**Confidence:** HIGH — subprocess venv test pattern is well-established for isolation verification.

### Pattern 5: DigitalOcean App Platform CI via GitHub Actions

**What:** DigitalOcean App Platform triggers builds from GitHub pushes but does NOT provide native CI hooks. GitHub Actions is the correct CI layer for pre-deploy PR checks. The app.yaml already specifies `deploy_on_push: true` — GitHub Actions runs before the deploy.

**Key insight:** The CI workflow must run in GitHub Actions (`.github/workflows/ci.yml`), not inside DigitalOcean's build step. DigitalOcean build commands are only for deployment — they are not a PR gate.

```yaml
# .github/workflows/ci.yml
name: CI
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  python-isolation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v4
      - name: Install workspace
        run: uv sync --workspace
      - name: Run isolation test
        run: cd packages/efofx-shared && uv run pytest tests/test_isolation.py -v
      - name: Run full pytest suite
        run: cd apps/efofx-estimate && uv run pytest -v

  typescript-build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm install
      - name: Build shared UI package
        run: cd packages/efofx-ui && npm run build
      - name: Build widget
        run: cd apps/efofx-widget && npm run build
      - name: Build dashboard
        run: cd apps/efofx-dashboard && npm run build
```

**Confidence:** MEDIUM — GitHub Actions syntax is well-known; `astral-sh/setup-uv` action is the correct current method for uv in CI (verified against uv documentation).

### Anti-Patterns to Avoid

- **Importing `app.*` modules in efofx-shared:** Any `from app.core.config import settings` in shared code breaks isolation. Shared code must receive config values as parameters, not read them from `app.*`.
- **Re-exporting estimation-domain types:** `EstimationOutput`, `ChatMessage` from `widget.d.ts` reference estimation-specific structures. These stay in `apps/efofx-widget/src/types/` — do NOT move to `@efofx/ui`. Only pure UI components with no domain knowledge move.
- **Shared package depending on FastAPI:** `file_utils.py` imports `fastapi.UploadFile` — this file CANNOT move to efofx-shared without stripping the FastAPI import. It stays in the app or the FastAPI import gets replaced with a protocol/interface.
- **Global CSS class names in CSS modules:** The current components use `efofx-bubble-wrapper` as global class names. Migrating to CSS modules requires renaming to camelCase local names AND updating every reference. Don't mix global and module scopes.
- **TypeScript strict mode mismatch:** `packages/efofx-ui` must use the same `strict` settings as the consuming apps (`~5.9.3`). A looser tsconfig in the shared package produces a false sense of type safety.

## Codebase Audit: What Goes Where

Based on reading the actual code, here is the pre-determined boundary map for EXTR-01:

### Python — Extract to `packages/efofx-shared/`

| Module | Current Location | Extract? | Rationale |
|--------|-----------------|----------|-----------|
| `app/utils/crypto.py` | `apps/efofx-estimate/` | YES | Pure stdlib + cryptography; no app imports; already tested in `tests/utils/test_crypto.py` |
| `app/utils/calculation_utils.py` | `apps/efofx-estimate/` | EVALUATE | No app imports, but domain logic (pool costs, region multipliers) — may be too estimation-specific |
| `app/utils/validation_utils.py` | `apps/efofx-estimate/` | PARTIAL | Imports `app.core.constants` (Region, ReferenceClassCategory enums) — extract only after extracting constants |
| `app/core/constants.py` | `apps/efofx-estimate/` | PARTIAL | Pure Python enums (Region, ReferenceClassCategory) are reusable; API_MESSAGES are app-specific. Extract the enums only. |
| `app/models/_objectid.py` | `apps/efofx-estimate/` | EVALUATE | Likely a MongoDB ObjectId serializer — check if it imports motor/pymongo |
| `app/utils/file_utils.py` | `apps/efofx-estimate/` | NO | Imports `fastapi.UploadFile` — cannot extract without architectural change |
| All services, API, DB modules | `apps/efofx-estimate/` | NO | Tightly coupled to FastAPI, Motor, tenant system |

**Note:** `apps/estimator-project/` is an older prototype (different pyproject.toml, missing v1.1 services). The extraction source is exclusively `apps/efofx-estimate/`.

### TypeScript — Extract to `packages/efofx-ui/`

| Component | Current Location | Extract? | Rationale |
|-----------|----------------|----------|-----------|
| `ChatBubble.tsx` | `efofx-widget/src/components/` | YES | Pure presentational; receives `ChatMessage` type (role + content string); no hooks |
| `EstimateCard.tsx` | `efofx-widget/src/components/` | YES | Uses local `useState` only; imports `EstimationOutput` type — must move type with component OR define a local interface |
| `TypingIndicator.tsx` | `efofx-widget/src/components/` | YES | Zero props, zero imports; purest extraction candidate |
| `ErrorBoundary.tsx` | `efofx-widget/src/components/` | YES | Generic React error boundary; no domain logic |
| `LoadingSkeleton.tsx` | `efofx-dashboard/src/components/` | YES | Generic skeleton UI; no calibration-specific logic |
| `ChatPanel.tsx` | `efofx-widget/src/components/` | NO | Uses `useChat` hook and widget-specific state |
| `FloatingButton.tsx` | `efofx-widget/src/components/` | NO | Widget-specific UI element |
| `ConsultationCTA.tsx` | `efofx-widget/src/components/` | NO | Estimation domain; links to ConsultationForm |
| `ConsultationForm.tsx` | `efofx-widget/src/components/` | NO | Estimation domain; uses `useChat` hook |
| `LeadCaptureForm.tsx` | `efofx-widget/src/components/` | NO | Estimation domain |
| `NarrativeStream.tsx` | `efofx-widget/src/components/` | NO | SSE streaming logic; widget-specific |
| `ShadowDOMWrapper.tsx` | `efofx-widget/src/components/` | NO | Widget-embed mechanism; not a reusable UI component |
| All dashboard-specific components (AccuracyBucketBar, CalibrationMetrics, etc.) | `efofx-dashboard/src/components/` | NO | Calibration domain; depend on Recharts |

**Type resolution for EstimateCard:** `EstimationOutput` is defined in `apps/efofx-widget/src/types/widget.d.ts`. The extracted `EstimateCard` should define a local `EstimationOutput` interface in its own file within `packages/efofx-ui/` — this avoids a cross-package type dependency on the widget app.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Python monorepo linking | Custom PYTHONPATH hacks or `pip install -e` in `.env` | uv workspace with `{ workspace = true }` dependency | uv handles editable installs, lock files, and venv isolation automatically |
| npm cross-package linking | Symlinks, custom copy scripts | npm `workspace:*` protocol | npm workspaces handles symlinks, hoisting, and version resolution |
| Fresh-venv import test | Manual shell scripts checking pip list | pytest + `venv` stdlib + `subprocess` | Self-contained, repeatable, works in any CI environment |
| CSS class isolation | Custom CSS-in-JS solution | CSS modules (`.module.css`) | Already supported by vite; no new dependency; class names auto-scoped |
| TypeScript declarations for shared package | Manual `.d.ts` writing | TypeScript project references or `tsc --declaration` | `vite build --mode lib` with TypeScript generates declarations automatically |

**Key insight:** The hardest part of this extraction is NOT the file moves. It is identifying which imports in moved files point back to `app.*` and surgically removing that coupling. Map all imports before moving anything.

## Common Pitfalls

### Pitfall 1: Circular Import Chain After Extraction

**What goes wrong:** `efofx-shared` imports `pydantic` model, which is also used by `app.models.*`, which imports `app.core.constants`. After extraction, if `efofx-shared` also imports `app.core.constants`, you have a circular dependency at the package level.

**Why it happens:** Moving individual modules without tracing the full import graph first.

**How to avoid:** Run `python -m modulefinder apps/efofx-estimate/app/utils/crypto.py` before extraction to see all transitive imports. Only move modules whose entire transitive import graph contains no `app.*` references.

**Warning signs:** `ModuleNotFoundError: No module named 'app'` when running the isolation test.

### Pitfall 2: EstimationOutput Type Coupling in EstimateCard

**What goes wrong:** `EstimateCard.tsx` imports `EstimationOutput` from `../types/widget`. After moving `EstimateCard` to `@efofx/ui`, this relative import breaks. If you fix it by importing from `@efofx-widget` instead, you've created a cross-package dependency that defeats the purpose.

**Why it happens:** Types are treated as secondary concerns during extraction.

**How to avoid:** Define a standalone `EstimationOutput` interface in `packages/efofx-ui/src/types/estimation.ts`. The widget app's `types/widget.d.ts` can then re-export from `@efofx/ui` or keep its own copy (acceptable since types are structural, not runtime).

**Warning signs:** TypeScript `tsc` errors about relative imports crossing package boundaries.

### Pitfall 3: CSS Global Names Leaking into CSS Modules

**What goes wrong:** `ChatBubble` uses class names like `efofx-bubble-wrapper`. When converted to CSS modules, the component reference becomes `styles.efofx-bubble-wrapper` — which is invalid JS property syntax (hyphen in property name). You must use `styles['efofx-bubble-wrapper']` or rename.

**Why it happens:** CSS modules expect camelCase property access in JSX.

**How to avoid:** During extraction, rename all CSS class names to camelCase in both the `.module.css` file and the component JSX. `efofx-bubble-wrapper` becomes `bubbleWrapper`.

**Warning signs:** TypeScript errors on `styles.efofx-*` property access; runtime CSS not applying.

### Pitfall 4: uv Workspace Not Found

**What goes wrong:** Running `uv run pytest` in `apps/efofx-estimate/` cannot find `efofx-shared` because `uv.toml` is at the workspace root but the app's `pyproject.toml` declares `efofx-shared` as a workspace dependency.

**Why it happens:** uv workspace resolution requires the command to be run from the workspace root OR for uv to detect the workspace root by walking up the directory tree.

**How to avoid:** Run all uv commands from the workspace root OR confirm uv's workspace-root auto-detection is working. Document this in the README.

**Warning signs:** `Cannot find package 'efofx-shared'` during `uv sync` inside `apps/efofx-estimate/`.

### Pitfall 5: DigitalOcean Build Command Cannot Access Workspace Root

**What goes wrong:** `apps/efofx-estimate/.do/app.yaml` has `build_command: pip install -r requirements.txt` and `source_dir: /apps/efofx-estimate`. After extraction, `efofx-shared` lives at `/packages/efofx-shared/` — outside the `source_dir`. DigitalOcean only uploads the `source_dir` subtree, so `efofx-shared` is unavailable.

**Why it happens:** DigitalOcean App Platform sends only the `source_dir` to the build worker.

**How to avoid:** Change `source_dir` to `/` (workspace root) or add `efofx-shared` as a `pip install -e ./packages/efofx-shared` step in the build command. The workspace-root `source_dir` approach is cleaner — DigitalOcean supports it.

**Warning signs:** `ModuleNotFoundError: No module named 'efofx_shared'` in production logs after extraction.

### Pitfall 6: `calculation_utils.py` and `validation_utils.py` Are Estimation-Domain Code

**What goes wrong:** These files look like generic utilities (calculation, validation) but their implementations are hardcoded for pool construction pricing regions and estimation-specific cost categories. Extracting them to `efofx-shared` creates a "shared" package that is actually estimation-specific, misleading future IT/dev vertical developers.

**Why it happens:** File names suggest generic utility; content is domain-specific.

**How to avoid:** Do NOT extract `calculation_utils.py` or `validation_utils.py` in their current form. Extract only `crypto.py` and the pure enum portion of `constants.py`. Boundary document should mark both files explicitly as "stays in apps/efofx-estimate — estimation domain".

**Warning signs:** Future IT/dev vertical importing `calculate_labor_cost` for software projects with pool-specific region multipliers.

## Code Examples

### uv Workspace Sync

```bash
# From workspace root — installs all workspace members
uv sync

# Verify efofx-shared is linked into efofx-estimate venv
uv run --directory apps/efofx-estimate python -c "import efofx_shared; print(efofx_shared.__version__)"
```

### npm Workspace Install

```bash
# From workspace root — installs all workspaces, creates symlinks
npm install

# Verify @efofx/ui is linked
ls apps/efofx-widget/node_modules/@efofx/ui  # should symlink to packages/efofx-ui
```

### Shared Package Barrel Export

```typescript
// packages/efofx-ui/src/index.ts
export { ChatBubble } from './components/ChatBubble/ChatBubble';
export type { ChatBubbleProps } from './components/ChatBubble/ChatBubble';
export { EstimateCard } from './components/EstimateCard/EstimateCard';
export type { EstimateCardProps, EstimationOutput } from './components/EstimateCard/EstimateCard';
export { TypingIndicator } from './components/TypingIndicator/TypingIndicator';
export { ErrorBoundary } from './components/ErrorBoundary/ErrorBoundary';
export { LoadingSkeleton } from './components/LoadingSkeleton/LoadingSkeleton';
```

### Consuming @efofx/ui in efofx-widget (after extraction)

```tsx
// apps/efofx-widget/src/components/ChatPanel.tsx (updated)
// Before: import { ChatBubble } from './ChatBubble';
import { ChatBubble, TypingIndicator } from '@efofx/ui';
```

### Python Isolation Test (minimal)

```python
# packages/efofx-shared/tests/test_isolation.py
import subprocess, sys, venv
from pathlib import Path

def test_no_app_imports_in_shared_package(tmp_path):
    venv.create(str(tmp_path / "env"), with_pip=True)
    pip = str(tmp_path / "env" / "bin" / "pip")
    python = str(tmp_path / "env" / "bin" / "python")
    pkg_dir = str(Path(__file__).parent.parent)

    r = subprocess.run([pip, "install", pkg_dir, "-q"], capture_output=True)
    assert r.returncode == 0, r.stderr.decode()

    r = subprocess.run([python, "-c", "import efofx_shared"], capture_output=True)
    assert r.returncode == 0, r.stderr.decode()

    for forbidden in ["fastapi", "motor", "uvicorn", "app"]:
        r = subprocess.run([python, "-c", f"import {forbidden}"], capture_output=True)
        assert r.returncode != 0, f"Prohibited import '{forbidden}' succeeded"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `pip install -e .` for local packages | `uv workspace` with `{ workspace = true }` | 2024 (uv 0.x → stable) | Lock file covers all workspace members; reproducible CI |
| Custom npm `link` scripts | `workspace:*` in package.json | npm 7+ (2020) | Native; no extra tooling |
| Separate tsconfig per project | TypeScript project references | TS 3.0+ | Incremental builds; shared types across packages |
| Copying shared CSS into each app | CSS modules co-located with component | Vite 2+ support | Zero runtime cost; class isolation by default |

**Deprecated/outdated:**
- `pip install -e ./packages/efofx-shared` in requirements.txt: Works but not workspace-aware; use `uv workspace` instead
- Storybook for component documentation: Out of scope per REQUIREMENTS.md — TypeScript types + JSDoc is the documented standard

## Open Questions

1. **DigitalOcean source_dir scope**
   - What we know: `app.yaml` currently has `source_dir: /apps/efofx-estimate` — this will NOT include `packages/efofx-shared/`
   - What's unclear: Whether changing to `source_dir: /` causes issues with DigitalOcean's health check path or environment variable scoping
   - Recommendation: Change `source_dir` to `/` (workspace root) and update `build_command` to `pip install -r apps/efofx-estimate/requirements.txt && pip install packages/efofx-shared/`. Planner should include a DigitalOcean app.yaml update task.

2. **uv workspace exact dependency syntax**
   - What we know: uv workspace supports local path dependencies; the exact syntax for workspace members may use `efofx-shared @ { workspace = true }` or a file path reference
   - What's unclear: Whether uv 0.x uses the exact syntax above or a different form
   - Recommendation: Planner's task should include "verify exact syntax from uv docs" as a step. Use `uv add --workspace efofx-shared` as the canonical way to add the dependency.

3. **`_objectid.py` extraction candidacy**
   - What we know: `apps/efofx-estimate/app/models/_objectid.py` likely handles MongoDB ObjectId serialization
   - What's unclear: Whether it imports `motor` or `pymongo` — if it does, it CANNOT go in efofx-shared
   - Recommendation: Planner should include reading this file in the audit task before making a boundary decision.

4. **Frontend TypeScript build mode for efofx-ui**
   - What we know: vite supports library mode (`vite build --mode lib`) for building packages with type declarations
   - What's unclear: Whether consuming apps should import `@efofx/ui` from source (TypeScript files directly) or from a built dist — source imports are simpler for a monorepo with no external consumers
   - Recommendation: Use source imports (point `main` to `src/index.ts`) for now. No build step needed in the shared package itself — consuming app's build tool resolves TypeScript directly.

## Sources

### Primary (HIGH confidence)
- Direct codebase read — `apps/efofx-estimate/app/utils/crypto.py`, `app/utils/calculation_utils.py`, `app/utils/validation_utils.py`, `app/utils/file_utils.py`, `app/core/constants.py`, `app/core/security.py`
- Direct codebase read — `apps/efofx-estimate/pyproject.toml`, `.do/app.yaml`, `tests/utils/test_crypto.py`, `tests/conftest.py`
- Direct codebase read — `apps/efofx-widget/src/components/ChatBubble.tsx`, `EstimateCard.tsx`, `TypingIndicator.tsx`, `package.json`
- Direct codebase read — `apps/efofx-dashboard/src/components/LoadingSkeleton.tsx`, `package.json`
- `.planning/phases/09-shared-library-extraction/09-CONTEXT.md` — locked decisions

### Secondary (MEDIUM confidence)
- npm workspaces documentation pattern — `workspace:*` protocol is well-established since npm 7 (2020)
- `astral-sh/setup-uv` GitHub Action — standard action for uv in CI; verified by uv project's own CI examples
- Python `venv` stdlib + `subprocess` isolation test — standard pattern for verifying clean package installs

### Tertiary (LOW confidence)
- uv workspace `{ workspace = true }` exact pyproject.toml syntax — training knowledge; should be verified against https://docs.astral.sh/uv/concepts/projects/workspaces/ before writing the task

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — based on direct codebase reads; both build backends (setuptools, npm) are already in use
- Architecture: HIGH for component extraction; MEDIUM for uv workspace config syntax
- Pitfalls: HIGH — CSS module naming pitfall and DigitalOcean source_dir issue are concrete findings from reading the actual code
- Boundary map: HIGH for "NO" decisions (file_utils, all dashboard domain components); MEDIUM for "EVALUATE" items (calculation_utils, _objectid.py)

**Research date:** 2026-03-15
**Valid until:** 2026-04-15 (uv workspace syntax should be re-verified at planning time; all other findings are stable)
