# Coding Conventions

**Analysis Date:** 2026-02-26

## Naming Patterns

**Files:**
- Python modules: `snake_case.py` (e.g., `calculation_utils.py`, `reference_class_model.py`)
- TypeScript files: camelCase or snake_case depending on context (e.g., `auth.js`, `auth.test.js`)
- React components: PascalCase (e.g., `App.tsx`)
- Test files: `<module>.test.py` or `<module>.test.js` (e.g., `test_reference_class_model.py`, `auth.test.js`)

**Functions:**
- Python: `snake_case` (e.g., `calculate_cost_breakdown()`, `apply_regional_adjustment()`)
- JavaScript/TypeScript: camelCase for functions and exports (e.g., `verifyHmac()`, `extractTenantId()`, `createRequestLogger()`)

**Variables:**
- Python: `snake_case` (e.g., `base_cost`, `region_multiplier`, `cost_breakdown`)
- JavaScript/TypeScript: camelCase (e.g., `baseMultiplier`, `hourlyRate`, `timestampSkew`)
- Constants: UPPER_SNAKE_CASE in both languages (e.g., `REGIONAL_ADJUSTMENTS`, `MAX_ESTIMATIONS_PER_MONTH`)

**Types/Classes:**
- Python Pydantic models: PascalCase (e.g., `ReferenceClass`, `CostDistribution`, `TenantUpdate`)
- Python utility classes: PascalCase (e.g., `Settings`, `PyObjectId`)
- TypeScript interfaces/types: PascalCase (if used)
- Enums: UPPER_SNAKE_CASE for values (e.g., `development: 0.70`, `testing: 0.20`)

## Code Style

**Formatting:**
- Python: Black (line length: 88 characters)
  - Config: `pyproject.toml` with `tool.black` section
  - Target version: Python 3.9+
- JavaScript/TypeScript: Prettier (line length: 80 characters)
  - Config: `.prettierrc` with settings for semi-colons, trailing commas (es5), single quotes
  - TabWidth: 2 spaces, no tabs
- React/TypeScript: Uses Vite + TypeScript 5.9+

**Linting:**
- Python: Black + Flake8 or Ruff
  - Flake8 config in `pyproject.toml`: max-line-length 88, excludes E203 W503
  - Ruff in newer projects: target Python 3.13, selects E F I N W B C4 UP ARG SIM TCH Q RUF
- JavaScript: ESLint
  - Modern flat config format (`eslint.config.js`)
  - Extends `js.configs.recommended`, `tseslint.configs.recommended`
  - React-specific: `reactHooks.configs['recommended-latest']`, `reactRefresh.configs.vite`
  - Rules: no-unused-vars (warn), prefer-const (error), no-var (error), no-console (off)

## Import Organization

**Order (Python):**
1. Standard library imports (e.g., `from typing import`, `from datetime import`)
2. Third-party imports (e.g., `from fastapi import`, `from pydantic import`)
3. Local application imports (e.g., `from app.models import`, `from app.core import`)

**Order (JavaScript/TypeScript):**
1. Node.js built-ins (e.g., `import crypto from 'node:crypto'`)
2. Third-party packages (e.g., `import pino from 'pino'`, `import jwt from 'jsonwebtoken'`)
3. Local modules (relative imports)

**Path Aliases:**
- Python: Uses absolute imports (e.g., `from app.models.tenant import Tenant`)
- TypeScript: May use path aliases (configured in `tsconfig.json`)

## Error Handling

**Patterns:**
- **Python (Pydantic models):** Use Pydantic's validation framework
  - Field validators in models (e.g., `@field_validator` decorators)
  - Custom validation logic (e.g., cost_breakdown_template must sum to 1.0 ± 1% tolerance)
  - Raises `ValidationError` for invalid data
  - Example: `test_cost_breakdown_sum_validation_*` tests in `test_reference_class_model.py`

- **Python (FastAPI):** Use HTTPException for API errors
  - Example: `from fastapi import HTTPException`
  - Configured with status_code and detail

- **JavaScript/TypeScript:** Return error objects with structured format
  - Pattern: `{ ok: false, reason: 'error_code' }` or `{ ok: true, data: value }`
  - Used in `auth.js`: `verifyHmac()`, `verifyJwt()` return `{ ok, reason }` structure
  - Try-catch for async operations, with error logging

## Logging

**Framework:**
- Python: Built-in `logging` module
  - Logger named per module: `logger = logging.getLogger(__name__)`
  - Setup in main: `logging.basicConfig(level=logging.INFO)`
  - Example: `app/main.py` configures logging and logs startup/shutdown events

- JavaScript/TypeScript: Pino logger
  - Configured in `lib/log.js`
  - Log level from environment: `process.env.LOG_LEVEL || 'info'`
  - Structured logging with context: `trace_id`, `tenant_id`, `tool`, `method`, `path`, `user_agent`

**Patterns:**
- Log at startup/shutdown lifecycle events
- Log errors with full stack traces: `logger.error({ ... error: error.message, stack: error.stack })`
- Add contextual information: trace IDs, tenant IDs, operation names
- Use child loggers for request context: `logger.child({ trace_id, tenant_id })`
- Log structured data (objects) not concatenated strings for better machine parsing

## Comments

**When to Comment:**
- Module docstring: Always include (triple-quoted in Python, JSDoc style in JS)
- Function docstring: Always include purpose, args, returns
- Complex business logic: Explain the "why" not the "what"
- Unusual patterns or workarounds: Document rationale
- Do NOT comment obvious code (e.g., `x = x + 1  # increment x`)

**JSDoc/TSDoc:**
- Python: Docstrings with parameter descriptions
  - Example: `"""Calculate cost breakdown based on template percentages."""`
  - Args documented: `base_cost: float, breakdown_template: Dict[str, float]`
  - Returns documented: `-> Dict[str, float]`

- JavaScript/TypeScript: JSDoc comments above functions
  - Use `/** ... */` format
  - Example: `/** Calculate confidence score based on reference data quality. */`

## Function Design

**Size:**
- Keep functions focused on single responsibility
- Python utility functions: 10-30 lines typical (e.g., `apply_regional_adjustment()` is 3 lines)
- Calculation functions: return single value or dict, don't mutate state
- No deep nesting (max 2-3 levels)

**Parameters:**
- Use type hints in Python (e.g., `def calculate_cost_breakdown(total_cost: float, breakdown_template: Dict[str, float])`)
- Use descriptive names, not abbreviations
- Keep parameter count under 5 (use dicts/objects for multiple related params)
- Optional parameters use `Optional[Type] = None` pattern

**Return Values:**
- Explicit return type hints in Python (e.g., `-> Dict[str, float]`)
- Return dictionaries for multiple related values (e.g., cost breakdown dict)
- Return tuples for multiple independent values (e.g., `Tuple[float, float, float]` for p50, p80, p95)
- Return objects with `{ ok: bool, reason?: str, data?: value }` structure in JavaScript

## Module Design

**Exports:**
- Python: Modules export public functions/classes, private helpers prefixed with `_`
- JavaScript: Explicit named exports (e.g., `export function verifyHmac()`)
- Use `from module import specific_name` not `import *`

**Barrel Files:**
- Python: `__init__.py` files can re-export for convenience
  - Example: `app/utils/__init__.py` may re-export common utilities
- JavaScript: Avoid deep barrel files, export directly from implementation files

**Organization:**
- Python: Group by domain/responsibility
  - `app/models/` - Pydantic models
  - `app/utils/` - Utility functions
  - `app/core/` - Core configuration and security
  - `app/api/` - API routes
  - `app/db/` - Database operations

- JavaScript: Similar domain-based organization
  - `lib/` - Core utilities (auth, logging, db, schemas)
  - `packages/` - Modular functionality by feature

---

*Convention analysis: 2026-02-26*
