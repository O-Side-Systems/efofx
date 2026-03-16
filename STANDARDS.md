# efOfX Code Quality Standards

This document defines the code style, structure, testing, documentation, error handling, and logging conventions for the efOfX workspace. All contributions must conform to these standards.

---

## 1. Code Style

### Python

- **Formatter:** [black](https://black.readthedocs.io/) 24.1.1, `line-length = 88`
- **Linter:** [flake8](https://flake8.pycqa.org/) 7.0.0, `max-line-length = 88`, `extend-ignore = E203, W503`
- **Type checker:** [mypy](https://mypy.readthedocs.io/) 1.8.0 in strict mode

Run before committing:
```bash
black apps/efofx-estimate/app/
flake8 apps/efofx-estimate/app/
mypy apps/efofx-estimate/app/
```

Configuration lives in `apps/efofx-estimate/pyproject.toml` under `[tool.black]`, `[tool.flake8]`, and `[tool.mypy]`.

Use `# noqa: E501` (with two spaces before `#`) for lines containing unavoidable long content (SSE payloads, regex patterns, long string literals). Do not use noqa for structural code — restructure instead.

### TypeScript / React

- **Linter:** ESLint (project-level config)
- **Type checker:** TypeScript ~5.9.3 in strict mode (`"strict": true` in tsconfig)
- **No formatter configured** — keep consistent whitespace by convention (2-space indent)

Run before committing:
```bash
npx tsc --noEmit          # from each app or package dir
npm run build             # verifies full build
```

### Naming Conventions

| Context | Convention | Example |
|---------|-----------|---------|
| Python variables, functions, modules | `snake_case` | `tenant_id`, `get_tenant()` |
| Python classes, Pydantic models | `PascalCase` | `TenantService`, `EstimationOutput` |
| Python constants | `UPPER_SNAKE_CASE` | `API_MESSAGES`, `DB_COLLECTIONS` |
| TypeScript variables, functions | `camelCase` | `tenantId`, `getEstimate()` |
| React components | `PascalCase` | `ChatBubble`, `EstimateCard` |
| CSS module class names | `camelCase` | `.bubbleWrapper`, `.rangeBar` |
| CSS custom properties | `--kebab-case` | `--brand-primary`, `--color-border` |

---

## 2. File and Folder Structure

### Python Apps (`apps/efofx-estimate/`)

```
app/
  api/          # FastAPI routers — thin, delegate to services
  core/         # Config, constants, rate limiting, security
  db/           # MongoDB connection, TenantAwareCollection
  middleware/   # Custom ASGI middleware
  models/       # Pydantic request/response models
  services/     # Business logic — one service class per domain
  templates/    # Jinja2 HTML templates (email, feedback form)
  utils/        # Pure utility functions (crypto, file handling, validation)
  main.py       # FastAPI app factory and lifespan
tests/
  unit/         # Fast, no I/O
  integration/  # Requires live MongoDB Atlas — excluded from dev runs
```

### Shared Python Package (`packages/efofx-shared/`)

```
efofx_shared/
  core/         # Pure domain constants (enums, config)
  utils/        # Pure utility functions (crypto, etc.)
tests/
  test_isolation.py   # Verifies zero app-server deps (fastapi/motor/uvicorn)
```

### React Apps (`apps/efofx-widget/`, `apps/efofx-dashboard/`)

```
src/
  components/   # Local components not suitable for shared package
  hooks/        # Custom React hooks
  types/        # TypeScript interfaces and types
  lib/          # API client, utilities
  pages/        # Top-level page components (dashboard only)
  main.tsx      # React entry point
```

### Shared UI Package (`packages/efofx-ui/`)

```
src/
  components/   # Shared React components, co-located CSS modules
    ComponentName/
      ComponentName.tsx
      ComponentName.module.css
  types/        # Shared TypeScript interfaces
  index.ts      # Barrel export
```

**Rules:**
- One component per file
- CSS modules co-located with their component (`Component.module.css` in same dir)
- No file exceeds ~300 lines — split into smaller modules when approaching that

---

## 3. Testing Patterns

### Python

- **Runner:** pytest 8.4.1
- **Async mode:** `asyncio_mode = "auto"` (configured in `pyproject.toml`)
- **Fixtures:** define in `conftest.py` at the appropriate scope level
- **Test file naming:** mirrors source structure — `app/services/auth_service.py` → `tests/unit/test_auth_service.py`

**Test naming convention:**
```
test_{what}_{condition}_{expected}
```
Examples:
- `test_consume_token_when_already_used_returns_false`
- `test_encrypt_with_valid_key_returns_encrypted_bytes`
- `test_isolation_no_fastapi_in_efofx_shared`

**Markers:**
```python
@pytest.mark.unit        # Fast, no I/O
@pytest.mark.integration # Requires live MongoDB Atlas
@pytest.mark.slow        # Long-running tests
```

Run only unit tests (development):
```bash
cd apps/efofx-estimate && python -m pytest -m "not integration" -v
```

**Isolation tests for shared packages:**
- `packages/efofx-shared/tests/test_isolation.py` verifies that `efofx_shared` can be imported in a fresh venv without `fastapi`, `motor`, or `uvicorn` present
- CI runs this test on every PR

### Frontend

No automated test suite currently exists for React apps. This is a documented gap. Future testing should use Vitest + React Testing Library. No Storybook (out of scope per REQUIREMENTS.md).

---

## 4. Documentation Standards

### Python Docstrings

Use **Google style** docstrings on all public functions and methods:

```python
def encrypt_value(value: str, key: bytes) -> str:
    """Encrypt a string value using Fernet HKDF.

    Args:
        value: The plaintext string to encrypt.
        key: The raw encryption key bytes.

    Returns:
        Base64-encoded ciphertext string.

    Raises:
        ValueError: If value is empty or key is invalid length.
    """
```

- Private methods (`_name`) may use a one-line docstring
- Module-level docstrings required for all non-trivial modules
- Class docstrings required for all service classes

### TypeScript / JSDoc

Document exported functions and non-obvious interface fields:

```typescript
/**
 * Fetches calibration trend data for the last N months.
 * @param months - Number of months to include in the trend window
 */
export function useCalibrationTrend(months: number) { ... }

interface CalibrationMetrics {
  /** null when below minimum threshold (10 outcomes) */
  by_reference_class: ReferenceClassMetrics[] | null;
}
```

---

## 5. Error Handling Patterns

### Python

- Use **specific exception types** — never bare `except:`
- Catch the narrowest exception type that makes sense
- Log the exception with context before re-raising or handling

```python
# Good
try:
    result = await collection.find_one({"tenant_id": tenant_id})
except pymongo.errors.OperationFailure as e:
    logger.error("MongoDB query failed for tenant %s: %s", tenant_id, e)
    raise

# Bad
try:
    result = await collection.find_one(...)
except:   # noqa — never do this
    pass
```

### API Error Responses

All API error responses use a consistent shape:
```json
{"error": "Human-readable message"}
```
With appropriate HTTP status codes:
- `400` — validation error, bad input
- `401` — missing or invalid authentication
- `403` — authenticated but not authorized
- `404` — resource not found
- `422` — well-formed request but business rule violation
- `429` — rate limit exceeded
- `500` — unexpected server error

### Frontend

- **ErrorBoundary** at layout level — wraps the entire app, catches render errors
- **Toast notifications** for transient errors (API failures, network errors)
- Never swallow errors silently — always log or surface to user
- Use the shared `ErrorBoundary` component from `@efofx/ui`:

```tsx
import { ErrorBoundary } from '@efofx/ui';

<ErrorBoundary>
  <App />
</ErrorBoundary>
```

---

## 6. Logging Conventions

### Python

Use the **stdlib `logging` module** — never `print()` in application code.

```python
import logging
logger = logging.getLogger(__name__)
```

**Log levels:**

| Level | When to use |
|-------|-------------|
| `DEBUG` | Detailed internal state — for development tracing only |
| `INFO` | Normal operations — service started, request processed, cache hit |
| `WARNING` | Degraded but functional — low-confidence RCF match, cache miss, optional service unavailable |
| `ERROR` | Failures that affect the user — DB error, external API failure, unhandled exception |
| `CRITICAL` | System-level failures — application cannot continue |

**Include tenant context** in log messages where applicable:
```python
logger.info("Generated estimate for tenant %s (session %s)", tenant_id, session_id)
logger.error("LLM request failed for tenant %s: %s", tenant_id, error)
```

Use `%s` formatting (not f-strings) in logger calls to defer string interpolation.

---

## 7. Import Conventions

### Python — Shared Package Imports

Always use the public package path, never relative or internal paths:

```python
# Good
from efofx_shared.utils.crypto import encrypt_value, decrypt_value
from efofx_shared.core.constants import EstimationStatus, Region

# Bad — do not use internal package structure directly
from packages.efofx_shared.efofx_shared.utils.crypto import ...
```

Re-export shims in `apps/efofx-estimate/app/` preserve backward compatibility:
```python
# app/utils/crypto.py — shim only
from efofx_shared.utils.crypto import *  # noqa: F401,F403
```

### TypeScript — Shared UI Imports

```typescript
// Good
import { ChatBubble, EstimateCard, ErrorBoundary } from '@efofx/ui';

// Bad — do not import from internal package paths
import ChatBubble from '../../packages/efofx-ui/src/components/ChatBubble/ChatBubble';
```

### Import Ordering (Python)

Follow PEP 8 import ordering (enforced by black/isort):
1. Standard library imports
2. Third-party imports
3. Local application imports

Group with a blank line between each section.

### No Circular Imports

Packages must not import from each other in a cycle:
- `efofx_shared` must not import from `apps/efofx-estimate`
- `@efofx/ui` must not import from `apps/efofx-widget` or `apps/efofx-dashboard`
- Cross-package imports flow in one direction: apps → shared packages

---

## CI Enforcement

All standards are enforced in CI on every PR to `main`:

- **Python isolation:** `pytest packages/efofx-shared/tests/test_isolation.py`
- **Python test suite:** `pytest apps/efofx-estimate/`
- **TypeScript type check:** `tsc --noEmit` on `packages/efofx-ui`
- **Frontend builds:** `npm run build` for both `efofx-widget` and `efofx-dashboard`

See `.github/workflows/ci.yml` for the full pipeline.
