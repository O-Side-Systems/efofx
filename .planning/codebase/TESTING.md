# Testing Patterns

**Analysis Date:** 2026-02-26

## Test Framework

**Runner:**
- Python: pytest 8.4.1 (for `efofx-estimate`)
  - Config: `pyproject.toml` under `[tool.pytest.ini_options]`
  - Location: `/Users/brettlee/work/efofx-workspace/apps/efofx-estimate/pyproject.toml`

- JavaScript: Vitest
  - Config: `vitest.config.js` in `estimator-mcp-functions`
  - Location: `/Users/brettlee/work/efofx-workspace/apps/estimator-mcp-functions/vitest.config.js`

**Assertion Library:**
- Python: Pytest's built-in assertions + Pydantic's `ValidationError` for model validation
- JavaScript: Vitest's expect API (e.g., `expect(value).toBe()`, `expect(value).toBeUndefined()`)

**Run Commands:**
```bash
# Python - Run all tests
pytest

# Python - Run with coverage
pytest --cov

# Python - Watch mode (not standard, use pytest-watch or manual rerun)
pytest -v --tb=short

# JavaScript - Run all tests
vitest run

# JavaScript - Watch mode
vitest

# JavaScript - Coverage
vitest --coverage
```

## Test File Organization

**Location:**
- Python: Co-located with code under `tests/` directory at root of project
  - Path: `/Users/brettlee/work/efofx-workspace/apps/efofx-estimate/tests/`
  - Structure: One test file per module (e.g., `test_reference_class_model.py`)

- JavaScript: Tests can be co-located or in `__tests__` directories
  - Pattern: `<module>.test.js` or `<module>.spec.js`
  - Config in `vitest.config.js`: includes `['**/*.test.js', '**/*.spec.js']`
  - Exclude: `node_modules/**`, `packages/**/node_modules/**`

**Naming:**
- Python: `test_<module_name>.py` (e.g., `test_reference_class_model.py`)
- JavaScript: `<module>.test.js` (e.g., `auth.test.js`)

**Structure:**
```
apps/efofx-estimate/
├── tests/
│   ├── __init__.py
│   ├── conftest.py              # Shared fixtures
│   └── test_reference_class_model.py
├── app/
│   ├── models/
│   ├── utils/
│   └── core/
```

## Test Structure

**Suite Organization (Python):**
```python
"""
Tests for Reference Class Pydantic models.

Tests validation rules, especially cost_breakdown_template percentage validation.
"""

import pytest
from pydantic import ValidationError
from app.models.reference_class import ReferenceClass

def test_valid_reference_class():
    """Test creating a valid reference class."""
    data = { ... }
    ref_class = ReferenceClass(**data)
    assert ref_class.name == "Test Pool Class"
    assert ref_class.category == "construction"

def test_cost_breakdown_sum_validation_exact():
    """Test that cost breakdown percentages must sum to 1.0 exactly."""
    # Test data
    data = { ... }
    # Should pass
    ref_class = ReferenceClass(**data)
    assert ref_class is not None
```

**Suite Organization (JavaScript):**
```javascript
import { describe, it, expect, beforeEach } from 'vitest';
import { verifyHmac, extractTenantId } from './auth.js';

describe('Auth Module', () => {
  let mockEvent;
  let mockSecret;

  beforeEach(() => {
    // Setup before each test
    mockSecret = Buffer.from('test-secret-key').toString('base64');
    mockEvent = { /* ... */ };
  });

  describe('extractTenantId', () => {
    it('should extract tenant_id from event body', () => {
      const event = { body: { tenant_id: 'acme-co' } };
      expect(extractTenantId(event)).toBe('acme-co');
    });

    it('should return undefined when tenant_id is missing', () => {
      const event = { body: { other: 'data' } };
      expect(extractTenantId(event)).toBeUndefined();
    });
  });
});
```

**Patterns:**
- **Module docstring:** Always include at top of test file
- **Test function names:** Clear, descriptive names starting with `test_` (Python) or `it('should...')` (JavaScript)
- **Arrange-Act-Assert:** Structure tests with clear sections (setup data, call function, verify results)
- **One assertion per test:** Focus on single behavior, though multiple related assertions allowed
- **Setup:** Use `conftest.py` fixtures (Python) or `beforeEach` hooks (JavaScript)
- **Teardown:** Handled by fixture cleanup or after hooks

## Mocking

**Framework:**
- Python: Pytest's `monkeypatch` fixture and unittest.mock (built-in)
- JavaScript: Vitest provides mocking via `vi` object (imported from `vitest`)

**Patterns (JavaScript example from `auth.test.js`):**
```javascript
describe('verifyHmac', () => {
  it('should return missing_headers when required headers are missing', () => {
    const event = { http: { headers: {} } };
    const result = verifyHmac({ event, secretBase64: mockSecret });
    expect(result.ok).toBe(false);
    expect(result.reason).toBe('missing_headers');
  });

  it('should return timestamp_skew when timestamp is too old', () => {
    const oldTimestamp = Math.floor((Date.now() / 1000) - 300).toString();
    mockEvent.http.headers['x-efofx-timestamp'] = oldTimestamp;

    const result = verifyHmac({ event: mockEvent, secretBase64: mockSecret });
    expect(result.ok).toBe(false);
    expect(result.reason).toBe('timestamp_skew');
  });
});
```

**What to Mock:**
- External service calls (API calls, database connections)
- Time-dependent operations (use fixed timestamps in tests)
- Cryptographic functions (use test keys/secrets)
- Environment variables (use process.env mocking)

**What NOT to Mock:**
- Validation logic (test the actual validators)
- Pure calculation functions (test with real math)
- Core business logic (test actual implementations)
- Internal module functions unless they're integration points

## Fixtures and Factories

**Test Data (Python example from `conftest.py`):**
```python
@pytest.fixture
def sample_tenant():
    """Sample tenant data for testing."""
    return {
        "name": "Test Construction Co",
        "api_key": "sk_test_123456789",
        "openai_api_key": "sk-openai-test-key",
        "regions": ["SoCal - Coastal", "NorCal - Bay Area"],
        "max_estimations_per_month": 1000,
        "is_active": True,
        "settings": {
            "default_confidence_threshold": 0.7,
            "enable_image_upload": True
        }
    }

@pytest.fixture
def sample_reference_class():
    """Sample reference class data for testing."""
    return {
        "tenant_id": None,
        "category": "construction",
        "subcategory": "pool",
        "name": "Test Pool Class",
        # ... full data structure
    }
```

**Location:**
- Python: `tests/conftest.py` - Shared fixtures imported automatically by pytest
  - Path: `/Users/brettlee/work/efofx-workspace/apps/efofx-estimate/tests/conftest.py`
  - Scope options: `session`, `module`, `function` (default)

- JavaScript: Inline in test files or in shared test utils
  - Use `beforeEach()` for per-test setup
  - Use `describe()` block scope for shared data

## Coverage

**Requirements:**
- Not enforced in codebase currently
- Target should be 80%+ for critical paths

**View Coverage:**
```bash
# Python
pytest --cov=app --cov-report=html

# JavaScript
vitest --coverage
```

**Configuration:**
- Python: `pyproject.toml` under `[tool.coverage.run]` and `[tool.coverage.report]`
  - Source: `["app"]`
  - Omit patterns: `*/tests/*`, `*/test_*`, `*/__pycache__/*`
  - Exclude lines: pragma no cover, abstractmethod, __repr__, etc.

## Test Types

**Unit Tests:**
- Scope: Test individual functions/methods in isolation
- Example: `test_valid_reference_class()` in `test_reference_class_model.py`
- Approach: Call function with known inputs, verify outputs
- Markers (pytest): `@pytest.mark.unit`

**Integration Tests:**
- Scope: Test multiple components working together
- Example: Test database operations, API endpoint chains
- Approach: Set up real test database (or in-memory), verify end-to-end flow
- Markers (pytest): `@pytest.mark.integration`
- Example: `test_db` fixture in `conftest.py` sets up MongoDB test database

**E2E Tests:**
- Framework: Not currently used in codebase
- Could be added: Use Cypress, Playwright, or similar for widget tests

**Slow Tests:**
- Marker: `@pytest.mark.slow`
- Run without: `pytest -m "not slow"`
- For tests that make external API calls or heavy database operations

## Common Patterns

**Async Testing (Python):**
```python
# pytest-asyncio handles async automatically
# conftest.py defines event_loop fixture for async tests

@pytest.fixture(scope="session")
def event_loop():
    """Create an instance of the default event loop for the test session."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()

@pytest.fixture
async def client(test_db):
    """Create test client with database connection."""
    from app.main import app
    with TestClient(app) as test_client:
        yield test_client
```

**Error Testing (Python):**
```python
def test_cost_breakdown_sum_validation_fails_over():
    """Test that percentages summing to > 1.0 fail validation."""
    data = { ... }  # Invalid data (sums > 1.0)

    with pytest.raises(ValidationError) as exc_info:
        ReferenceClass(**data)

    assert "must sum to 1.0" in str(exc_info.value)
```

**Error Testing (JavaScript):**
```javascript
it('should return bad_signature for invalid signature', () => {
  mockEvent.http.headers['x-efofx-signature'] = 'invalid-signature';
  const result = verifyHmac({ event: mockEvent, secretBase64: mockSecret });
  expect(result.ok).toBe(false);
  expect(result.reason).toBe('bad_signature');
});
```

**Parameterized Tests (Python):**
```python
@pytest.mark.parametrize("project_complexity,expected_multiplier", [
    ("simple", 0.8),
    ("standard", 1.0),
    ("complex", 1.3),
    ("very_complex", 1.6)
])
def test_complexity_multiplier(project_complexity, expected_multiplier):
    result = calculate_timeline_multiplier("pool", project_complexity, "SoCal - Coastal")
    assert result == expected_multiplier * 1.1  # Apply region multiplier
```

## Test Configuration

**pytest.ini_options (Python):**
```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py"]
python_classes = ["Test*"]
python_functions = ["test_*"]
addopts = [
    "--strict-markers",
    "--strict-config",
    "--verbose",
]
markers = [
    "slow: marks tests as slow (deselect with '-m \"not slow\"')",
    "integration: marks tests as integration tests",
    "unit: marks tests as unit tests",
]
asyncio_mode = "auto"
```

**Vitest configuration (JavaScript):**
```javascript
export default defineConfig({
  test: {
    globals: true,           // Use global describe/it/expect
    environment: 'node',     // Node.js environment
    include: ['**/*.test.js', '**/*.spec.js'],
    exclude: ['node_modules/**', 'packages/**/node_modules/**']
  }
});
```

---

*Testing analysis: 2026-02-26*
