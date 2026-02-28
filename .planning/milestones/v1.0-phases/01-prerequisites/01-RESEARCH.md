# Phase 1: Prerequisites - Research

**Researched:** 2026-02-26
**Domain:** Python dependency migration, bug fixes, DigitalOcean runtime configuration
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Patch queries only** — add tenant_id filter to all rcf_engine.py queries. Minimal change. Phase 2 builds the proper TenantAwareCollection middleware.
- **Automated test required** — write a test that creates two tenants, inserts data, and asserts zero cross-tenant leakage. Runs in CI.
- **Platform data in separate collection** — synthetic reference classes live in their own MongoDB collection, distinct from tenant-owned data.
- **Auto-include platform data** — queries automatically merge tenant data + platform reference data. No opt-in parameter needed.
- **No Redis/Valkey** — remove all existing Redis client code, connection config, and caching logic. MVP runs without Redis. Reserve the option to add it later for performance.
- **MongoDB-based rate limiting** for Phase 2 — store counters in MongoDB with TTL indexes instead of Redis/Valkey.
- **Full refactor to OpenAI v2 patterns** — use structured outputs, new client patterns, proper error types. Don't just make old code compile.
- **Python 3.11 (unpinned)** — specify 3.11 in DigitalOcean App Platform config, let the platform pick the latest patch.
- **Stick to requirements only** — no additional dependency audit or cleanup beyond PRQT-01 through PRQT-05 (plus Redis removal).
- **Remove Redis entirely** — if Redis code exists, remove it. Clean slate. Don't leave dormant Redis code.
- **Estimate schema defined as Pydantic models** (required for OpenAI v2 structured outputs).
- **Full refactor to OpenAI v2 patterns** — structured outputs, new client patterns, proper error types.

### Claude's Discretion

- How much of the full domain-aware structured output schema to implement in Phase 1 vs. deferring to Phase 3 (assess what the RCF engine can support now)
- Exact Pydantic model structure for the estimate schema
- Test framework and test organization for the tenant isolation test
- Order of dependency replacements

### Deferred Ideas (OUT OF SCOPE)

- Multi-domain estimates — single domain per estimate only
- Contractor-created custom domains — platform-defined only for now
- Full dependency audit — defer to Phase 6
- Redis/Valkey for caching — add back if performance requires it
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PRQT-01 | Abandoned dependencies replaced (python-jose → PyJWT, passlib → pwdlib, openai v1 → v2) | PyJWT 2.11.0 is a near-drop-in for python-jose HS256; pwdlib 0.3.0 with `[bcrypt]` extra replaces passlib; openai 2.24.0 keeps chat.completions API stable with `beta.chat.completions.parse` for structured outputs |
| PRQT-02 | Cross-tenant data leak in rcf_engine.py fixed (tenant_id filtering on all queries) | rcf_engine.py line 194 uses `query = {"category": category}` with no tenant_id filter — confirmed bug via code inspection |
| PRQT-03 | DB_COLLECTIONS import NameError in security.py fixed | security.py uses `DB_COLLECTIONS` on lines 58 and 91 but never imports it; `DB_COLLECTIONS` is defined in `app.core.constants` |
| PRQT-04 | LLM parsing stub replaced with real OpenAI structured output | `_parse_estimation_response()` in llm_service.py lines 122–145 is hardcoded and ignores LLM response; requires `client.beta.chat.completions.parse()` with Pydantic model |
| PRQT-05 | Python runtime bumped to 3.11 in DO App Platform config | DO App Platform uses `runtime.txt` file (not app.yaml) to pin Python version; format is `python-3.11.14`; 3.11.0–3.11.14 are available on Ubuntu-22 buildpack |
</phase_requirements>

---

## Summary

Phase 1 is a pure bug-fix and dependency-replacement phase. The codebase has been directly inspected and all five bugs are confirmed. The fixes are straightforward: a missing import (PRQT-03), a missing WHERE clause in a MongoDB query (PRQT-02), a Python runtime config file (PRQT-05), three library swaps in requirements.txt and pyproject.toml (PRQT-01), and replacing a hardcoded stub with an actual OpenAI structured output call (PRQT-04).

The most complex work in this phase is PRQT-04 (OpenAI v2 structured output). The user decision requires a "full refactor to OpenAI v2 patterns" — not just making old code compile. This means defining proper Pydantic models for the estimate schema and using `client.beta.chat.completions.parse()`. The user also deferred the full domain-aware schema to a later assessment, so Phase 1 only needs a working structured output implementation for the estimate response shape; the domain-specific cost categories are Phase 3's concern.

The dependency chain for Plan 01-02 is: fix python-jose → PyJWT first (security.py is broken without it, and PRQT-03 touches the same file), then passlib → pwdlib, then openai v1 → v2 (PRQT-04 builds on this), then update Python runtime. The `asyncio_mode = "auto"` in pyproject.toml and the existing pytest infrastructure mean the tenant isolation test (PRQT-02) can be written without any new test framework setup.

**Primary recommendation:** Fix PRQT-03 and PRQT-02 first (they unblock the backend from crashing), then do PRQT-01 dependency migrations, then implement PRQT-04 on top of the updated openai client, then PRQT-05.

---

## Confirmed Bugs (Direct Code Inspection)

### PRQT-03: DB_COLLECTIONS NameError
**File:** `apps/efofx-estimate/app/core/security.py`
**Lines:** 58, 91
**Root cause:** `DB_COLLECTIONS` is used in `validate_api_key()` and `get_current_tenant()` without being imported. The module imports from `app.core.constants` but only imports `API_MESSAGES` and `HTTP_STATUS`.
**Fix:** Add `DB_COLLECTIONS` to the import from `app.core.constants`.

```python
# Current (line 16-17 in security.py):
from app.core.constants import API_MESSAGES, HTTP_STATUS

# Fix:
from app.core.constants import API_MESSAGES, HTTP_STATUS, DB_COLLECTIONS
```

**Confidence:** HIGH — confirmed by direct code inspection of both files.

### PRQT-02: Cross-Tenant Query Bug in rcf_engine.py
**File:** `apps/efofx-estimate/app/services/rcf_engine.py`
**Line:** 194
**Root cause:** The query filters only by `category`, not by `tenant_id`. All tenants' reference classes are returned and scored. The code at lines 221–224 attempts to detect tenant-specific matches after the fact but cannot prevent data leakage.

```python
# Current (line 193-196 in rcf_engine.py):
query = {"category": category}
cursor = collection.find(query)

# Fix — filter by tenant data OR platform data (tenant_id=None):
query = {
    "category": category,
    "$or": [
        {"tenant_id": tenant_id},
        {"tenant_id": None}
    ]
}
```

The user decision requires: "queries automatically merge tenant data + platform reference data." The `$or` query is the correct minimal patch — it returns data belonging to this tenant plus platform data (tenant_id=None), preventing cross-tenant leakage.

**Confidence:** HIGH — confirmed by direct code inspection.

### PRQT-01 Library Status (confirmed via direct inspection)
- `python-jose[cryptography]==3.3.0` appears in both `requirements.txt` and `pyproject.toml`
- `PyJWT==2.8.0` is already in `pyproject.toml` dependencies but is NOT used (security.py imports from `jose`, not `jwt`)
- `passlib[bcrypt]==1.7.4` appears in `requirements.txt` but not in `pyproject.toml` — inconsistency
- `openai==1.51.0` is the current version (v1.x)
- No Redis code exists in `apps/efofx-estimate` — nothing to remove there

### PRQT-04: LLM Parsing Stub
**File:** `apps/efofx-estimate/app/services/llm_service.py`
**Lines:** 122–145 (`_parse_estimation_response`) and 147–166 (`_create_default_estimation`)
**Root cause:** `_parse_estimation_response()` ignores its `response: str` argument entirely and always returns hardcoded values. The method `generate_estimation()` calls it after an actual LLM call but discards the response.
**Fix:** Replace `generate_estimation()` and `_parse_estimation_response()` with a `client.beta.chat.completions.parse()` call using a Pydantic model as `response_format`.

### PRQT-05: Python Runtime
**File:** `apps/efofx-estimate/.do/app.yaml`
**Current state:** No Python version specified in app.yaml or runtime.txt. DigitalOcean defaults to 3.13.x when unspecified.
**Fix:** Create `apps/efofx-estimate/runtime.txt` with content `python-3.11.14`.
**Note:** Per DO docs, `runtime.txt` must be in the repository root of the source directory, which is `apps/efofx-estimate/` (per `source_dir: /apps/efofx-estimate` in app.yaml).

---

## Standard Stack

### Core (for this phase)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| PyJWT | 2.11.0 | JWT encode/decode | Actively maintained, simple API, replaces python-jose for HS256 use case |
| pwdlib | 0.3.0 | Password hashing | Created specifically as passlib replacement for modern Python (3.10+) |
| openai | >=2.20.0 | OpenAI API client | Required for structured output Pydantic parse; 2.24.0 is latest as of 2026-02-24 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cryptography | >=41.0.0 | Fernet encryption for BYOK | Already in requirements; needed by PyJWT `[crypto]` extra for RSA if needed |
| pytest | 8.4.1 | Test framework | Already configured; use for tenant isolation test |
| pytest-asyncio | 0.23.5 | Async test support | Already configured with `asyncio_mode = "auto"` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| pwdlib | argon2-cffi + bcrypt directly | More control but more boilerplate; pwdlib is cleaner API |
| PyJWT | joserfc | joserfc supports JWE but adds complexity not needed for HS256 JWT |

**Installation (updated requirements.txt):**
```
# Remove:
python-jose[cryptography]==3.3.0
passlib[bcrypt]==1.7.4
openai==1.51.0

# Add:
PyJWT==2.11.0
pwdlib[bcrypt]==0.3.0
openai>=2.20.0
```

---

## Architecture Patterns

### Pattern 1: PyJWT Migration from python-jose

**What:** Replace `from jose import jwt` with `import jwt`, adjust exception handling.
**When to use:** Everywhere python-jose was used.

```python
# Source: PyJWT 2.11.0 docs (pyjwt.readthedocs.io)

# Old (python-jose):
from jose import jwt
from jose.exceptions import JWTError

encoded = jwt.encode({"sub": "tenant_id"}, secret, algorithm="HS256")
payload = jwt.decode(token, secret, algorithms=["HS256"])
# except JWTError

# New (PyJWT):
import jwt

encoded = jwt.encode({"sub": "tenant_id"}, secret, algorithm="HS256")
payload = jwt.decode(token, secret, algorithms=["HS256"])
# except jwt.PyJWTError (or jwt.InvalidTokenError, jwt.ExpiredSignatureError)
```

**Key difference:** PyJWT's `encode()` returns a `str` (not bytes) in v2. The decode API is nearly identical.

**security.py currently uses:** `from jose import jwt` — one line change plus exception type updates.

### Pattern 2: pwdlib Migration from passlib

**What:** Replace `passlib.context.CryptContext` with `pwdlib.PasswordHash`.
**When to use:** Any password hash/verify operation.

```python
# Source: github.com/frankie567/pwdlib

# Old (passlib):
from passlib.context import CryptContext
pwd_context = CryptContext(schemes=["bcrypt"], deprecated="auto")
hashed = pwd_context.hash("password")
verified = pwd_context.verify("password", hashed)  # True/False

# New (pwdlib):
from pwdlib import PasswordHash
password_hash = PasswordHash.recommended()  # or PasswordHash(("bcrypt",))
hashed = password_hash.hash("password")
verified = password_hash.verify("password", hashed)  # True/False
```

**Note:** pwdlib requires Python >=3.10. Since Phase 1 bumps to Python 3.11, this is satisfied.

### Pattern 3: OpenAI v2 Structured Output

**What:** Replace free-text prompt + manual parsing with Pydantic model passed as `response_format`.
**When to use:** Any LLM call that needs structured data back.

```python
# Source: developers.openai.com/cookbook/examples/structured_outputs_intro

from pydantic import BaseModel
from openai import AsyncOpenAI

client = AsyncOpenAI(api_key=settings.OPENAI_API_KEY)

class EstimationResult(BaseModel):
    total_cost: float
    timeline_weeks: int
    confidence_score: float
    assumptions: list[str]
    # ... etc

completion = await client.beta.chat.completions.parse(
    model="gpt-4o-2024-08-06",  # or gpt-4o-mini — structured outputs require these models
    messages=[
        {"role": "system", "content": "..."},
        {"role": "user", "content": prompt}
    ],
    response_format=EstimationResult,
)

result = completion.choices[0].message.parsed
# result.total_cost, result.timeline_weeks, etc. — fully typed
```

**Important model constraint:** Structured outputs with `response_format` require `gpt-4o-2024-08-06` or `gpt-4o-mini` (not plain `gpt-4`). The current config uses `gpt-4` — this must change to a structured-output-compatible model.

**Error handling pattern for openai v2:**
```python
from openai import OpenAIError, RateLimitError, APITimeoutError

try:
    completion = await client.beta.chat.completions.parse(...)
except RateLimitError:
    ...
except APITimeoutError:
    ...
except OpenAIError as e:
    ...
```

### Pattern 4: Tenant-Scoped rcf_engine Query

**What:** Add `$or` clause to filter by tenant_id OR platform data (tenant_id=None).

```python
# apps/efofx-estimate/app/services/rcf_engine.py

# Current (line 194):
query = {"category": category}

# Fixed:
query = {
    "category": category,
    "$or": [
        {"tenant_id": tenant_id},
        {"tenant_id": None}       # platform reference data
    ]
}
```

**When tenant_id is None** (unauthenticated or system call): query only platform data.
```python
if tenant_id:
    query["$or"] = [{"tenant_id": tenant_id}, {"tenant_id": None}]
# else: no $or needed — platform data has tenant_id=None, so all results are platform data
```

### Anti-Patterns to Avoid

- **Don't just change the import:** security.py already imports `PyJWT` in pyproject.toml but the code still uses `from jose import jwt`. The code must be updated to use `jwt.encode()` / `jwt.decode()` / `jwt.PyJWTError`.
- **Don't use `response_format={"type": "json_object"}`** for structured outputs — use the Pydantic model directly with `beta.chat.completions.parse()` to get typed access.
- **Don't keep `gpt-4` as the model** when using structured outputs — it won't work; use `gpt-4o-mini` or `gpt-4o-2024-08-06`.
- **Don't put runtime.txt in the workspace root** — it must be in `apps/efofx-estimate/` which is the `source_dir` in app.yaml.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JWT encode/decode | Custom base64+HMAC | PyJWT | Algorithm confusion attacks, claim validation, expiry handling |
| Password hashing | Custom bcrypt calls | pwdlib | Correct iteration count defaults, timing-safe comparison |
| Structured LLM output parsing | regex/JSON parsing of free-text | `client.beta.chat.completions.parse()` | Guaranteed schema conformance, typed return, no parse errors |
| Cross-tenant query safety | Application-level post-filter | MongoDB `$or` query | DB-level enforcement, cannot be bypassed by code bugs |

**Key insight:** The entire problem set in Phase 1 arises from hand-rolled solutions: a hand-rolled import (missing), a hand-rolled response parser (stub), hand-rolled JWT lib choice (abandoned). Use the canonical tools.

---

## Common Pitfalls

### Pitfall 1: python-jose still present after migration
**What goes wrong:** `python-jose` stays in requirements.txt while `PyJWT` is added — both coexist, old code still uses jose
**Why it happens:** Adding a library without removing the old one
**How to avoid:** Remove `python-jose[cryptography]` from BOTH `requirements.txt` AND `pyproject.toml`. Update the import in security.py in the same commit.
**Warning signs:** Both `jose` and `jwt` imported in security.py

### Pitfall 2: PyJWT encode returns str, not bytes
**What goes wrong:** Code that does `encoded_jwt.decode()` (treating return as bytes) breaks
**Why it happens:** In older PyJWT (pre-2.0) encode returned bytes; in 2.x it returns str
**How to avoid:** Just use the return value directly as a string — no `.decode()` call needed
**Warning signs:** `AttributeError: 'str' object has no attribute 'decode'`

### Pitfall 3: pwdlib Python version requirement
**What goes wrong:** pwdlib fails to install on Python 3.9
**Why it happens:** pwdlib requires Python >=3.10
**How to avoid:** PRQT-05 (Python 3.11) should be committed before or with PRQT-01. In the CI/CD pipeline, verify the runtime is 3.11 before attempting to install pwdlib.
**Warning signs:** pip install fails with "Requires-Python >=3.10"

### Pitfall 4: Structured outputs model incompatibility
**What goes wrong:** `client.beta.chat.completions.parse()` fails with model error
**Why it happens:** Structured outputs require `gpt-4o-2024-08-06` or `gpt-4o-mini`, not `gpt-4`
**How to avoid:** Update `OPENAI_MODEL` in config.py and app.yaml to `gpt-4o-mini` (or `gpt-4o-2024-08-06`)
**Warning signs:** API error "This model does not support structured outputs"

### Pitfall 5: runtime.txt location wrong
**What goes wrong:** DO App Platform ignores runtime.txt and continues using Python 3.13
**Why it happens:** runtime.txt placed in workspace root instead of app source directory
**How to avoid:** Place `runtime.txt` in `apps/efofx-estimate/` (the `source_dir` from app.yaml). Verify by checking the DO deployment logs for "Python version" lines.
**Warning signs:** Deployment uses Python 3.13.x despite runtime.txt existing

### Pitfall 6: Tenant isolation test needs real MongoDB
**What goes wrong:** Tenant isolation test is meaningless if mocked
**Why it happens:** Unit tests mock the DB, so the $or query is never actually executed
**How to avoid:** Write the test as an integration test using the `test_db` fixture in conftest.py (connects to a real MongoDB test instance). Use `pytest.mark.integration` marker.
**Warning signs:** Test passes but no actual queries run against MongoDB

### Pitfall 7: passlib still in requirements after pwdlib migration
**What goes wrong:** Both coexist, import ambiguity, unnecessary dependency
**Why it happens:** Not removing the old library from BOTH files
**How to avoid:** Remove `passlib[bcrypt]` from requirements.txt. Verify no other module imports from passlib.
**Warning signs:** `import passlib` succeeds when it shouldn't

---

## Code Examples

### Fix for security.py: DB_COLLECTIONS import

```python
# Source: Direct code inspection of app/core/security.py and app/core/constants.py

# In app/core/security.py, change line 16:
# OLD:
from app.core.constants import API_MESSAGES, HTTP_STATUS

# NEW:
from app.core.constants import API_MESSAGES, HTTP_STATUS, DB_COLLECTIONS
```

### Fix for security.py: PyJWT migration

```python
# Source: PyJWT 2.11.0 docs

# OLD:
from jose import jwt
from jose.exceptions import JWTError

class AuthService:
    def verify_token(self, token: str) -> Dict[str, Any]:
        try:
            payload = jwt.decode(token, self.secret_key, algorithms=[self.algorithm])
            return payload
        except jwt.PyJWTError:  # This was wrong — jose uses JWTError
            raise HTTPException(...)

# NEW:
import jwt  # PyJWT

class AuthService:
    def verify_token(self, token: str) -> Dict[str, Any]:
        try:
            payload = jwt.decode(token, self.secret_key, algorithms=[self.algorithm])
            return payload
        except jwt.InvalidTokenError:  # PyJWT base exception
            raise HTTPException(...)
```

### Fix for rcf_engine.py: Tenant-scoped query

```python
# Source: Direct code inspection of app/services/rcf_engine.py

# Replace lines 193-196:
# OLD:
query = {"category": category}
cursor = collection.find(query)

# NEW:
if tenant_id:
    query = {
        "category": category,
        "$or": [
            {"tenant_id": tenant_id},
            {"tenant_id": None}  # platform reference classes
        ]
    }
else:
    query = {"category": category, "tenant_id": None}  # platform only

cursor = collection.find(query)
```

### Tenant isolation test structure

```python
# Source: Project test patterns from TESTING.md + pytest-asyncio docs

import pytest
from app.services.rcf_engine import find_matching_reference_class

@pytest.mark.integration
async def test_no_cross_tenant_leakage(test_db):
    """PRQT-02: Tenant A cannot see Tenant B's reference classes."""
    collection = get_reference_classes_collection()

    # Insert Tenant A's reference class
    await collection.insert_one({
        "tenant_id": "tenant_a",
        "category": "construction",
        "name": "Tenant A Class",
        "keywords": ["pool", "residential"],
        "regions": ["SoCal - Coastal"],
        # ... required fields
    })

    # Insert Tenant B's reference class
    await collection.insert_one({
        "tenant_id": "tenant_b",
        "category": "construction",
        "name": "Tenant B Class",
        "keywords": ["pool", "residential"],
        "regions": ["SoCal - Coastal"],
        # ... required fields
    })

    # Query as Tenant A — must not see Tenant B's data
    result = await find_matching_reference_class(
        description="pool installation",
        category="construction",
        region="SoCal - Coastal",
        tenant_id="tenant_a"
    )

    if result:
        assert result["reference_class"]["tenant_id"] != "tenant_b", \
            "Cross-tenant leak: Tenant A received Tenant B's reference class"

    # Cleanup
    await collection.delete_many({"tenant_id": {"$in": ["tenant_a", "tenant_b"]}})
```

### OpenAI v2 structured output: Pydantic model for estimates

```python
# Source: developers.openai.com/cookbook/examples/structured_outputs_intro
# For PRQT-04 — Claude has discretion on exact schema

from pydantic import BaseModel
from typing import Optional
from openai import AsyncOpenAI

class CostBreakdown(BaseModel):
    category: str
    amount: float
    percentage: float

class EstimationOutput(BaseModel):
    total_cost_p50: float
    total_cost_p80: float
    timeline_weeks_p50: int
    timeline_weeks_p80: int
    cost_breakdown: list[CostBreakdown]
    confidence_score: float  # 0-100 numeric
    assumptions: list[str]
    adjustment_factors: dict[str, float]  # e.g., {"Urban premium": 1.15}

async def generate_structured_estimation(
    description: str,
    region: str,
    reference_class: str
) -> EstimationOutput:
    client = AsyncOpenAI(api_key=settings.OPENAI_API_KEY)

    completion = await client.beta.chat.completions.parse(
        model="gpt-4o-mini",  # Must support structured outputs
        messages=[
            {"role": "system", "content": ESTIMATION_SYSTEM_PROMPT},
            {"role": "user", "content": f"Description: {description}\nRegion: {region}"}
        ],
        response_format=EstimationOutput,
    )

    return completion.choices[0].message.parsed
```

### DigitalOcean runtime.txt

```
# File: apps/efofx-estimate/runtime.txt
# Source: docs.digitalocean.com/products/app-platform/reference/buildpacks/python/

python-3.11.14
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| python-jose for JWT | PyJWT 2.x | python-jose went dormant ~2022; CVE-2024-33664 (DoS), CVE-2025-61152 (alg=none bypass) | Security risk eliminated |
| passlib for password hashing | pwdlib 0.3.0 | passlib unmaintained; breaks on Python 3.13+ | Future-proofing; cleaner API |
| openai v1 free-text + manual parsing | openai v2 beta.chat.completions.parse with Pydantic | openai v1 → v2 migration September 2025 | LLM responses are now actually used (not stubbed) |
| No Python version pinning | runtime.txt with python-3.11.14 | DO defaults to 3.13 without it | Stable platform, pwdlib requirement satisfied |

**Deprecated/outdated:**
- `python-jose[cryptography]==3.3.0`: Has unpatched CVE-2025-61152 (JWT signature bypass via alg=none). Must be removed.
- `passlib[bcrypt]==1.7.4`: Incompatible with Python 3.13+; pwdlib is the community-recommended replacement.
- `openai==1.51.0`: v1.x; project requires `openai>=2.20.0`.
- `gpt-4` model: Does not support structured outputs; must change to `gpt-4o-mini` or `gpt-4o-2024-08-06`.

---

## Current State Assessment (Direct Code Inspection)

### Files to modify for each requirement:

**PRQT-03 (DB_COLLECTIONS NameError):**
- `apps/efofx-estimate/app/core/security.py` — add `DB_COLLECTIONS` to import (line 16-17)

**PRQT-02 (cross-tenant query bug):**
- `apps/efofx-estimate/app/services/rcf_engine.py` — update query at line 194 (add `$or` clause)
- `apps/efofx-estimate/tests/services/` — add `test_tenant_isolation.py`

**PRQT-01 (abandoned dependencies):**
- `apps/efofx-estimate/requirements.txt` — remove python-jose, passlib; add PyJWT>=2.11, pwdlib[bcrypt], openai>=2.20.0
- `apps/efofx-estimate/pyproject.toml` — same changes + update `requires-python = ">=3.11"`
- `apps/efofx-estimate/app/core/security.py` — replace `from jose import jwt` with `import jwt`; update exception types
- Grep for any remaining `passlib` imports across the codebase (none found in initial scan, but verify)

**PRQT-04 (LLM parsing stub):**
- `apps/efofx-estimate/app/services/llm_service.py` — replace `_parse_estimation_response()` stub and `generate_estimation()` with `client.beta.chat.completions.parse()` call
- New Pydantic models for structured estimate output (add to `app/models/` or inline in llm_service.py)
- `apps/efofx-estimate/app/core/config.py` — update default `OPENAI_MODEL` from `gpt-4` to `gpt-4o-mini`
- `apps/efofx-estimate/.do/app.yaml` — update `OPENAI_MODEL` env var value

**PRQT-05 (Python 3.11 runtime):**
- Create `apps/efofx-estimate/runtime.txt` with `python-3.11.14`
- `apps/efofx-estimate/pyproject.toml` — update `requires-python = ">=3.11"` and classifiers
- `apps/efofx-estimate/pyproject.toml` — update `[tool.black] target-version` and `[tool.mypy] python_version`

### Redis status:
No Redis imports, no Redis client code found in `apps/efofx-estimate/`. Nothing to remove from the main estimation service. (The `estimator-project` app has Redis in its stack.md but that app is separate and not in scope for this phase.)

---

## Open Questions

1. **Schema scope for PRQT-04**
   - What we know: User deferred domain-aware structured output schema to Phase 3 assessment. Phase 1 needs "real" structured output, not a stub.
   - What's unclear: Whether the EstimationOutput Pydantic model in Phase 1 should include P50/P80 cost category breakdowns (from the domain schema) or just top-level totals.
   - Recommendation: Implement a minimal but real schema (p50_cost, p80_cost, timeline ranges, assumptions, confidence_score) that the existing rcf_engine results can populate. The full domain-aware category breakdown from CONTEXT.md can be added in Phase 3 when the domain model is built out.

2. **test_db fixture requires live MongoDB**
   - What we know: The existing `conftest.py` connects to a real MongoDB test instance (`efofx_estimate_test`). The tenant isolation test needs real queries.
   - What's unclear: Whether CI has a MongoDB instance available, or if we need mongomock.
   - Recommendation: Use the existing `test_db` fixture (real MongoDB). Flag in the test that it requires `MONGO_URI` env var. Use `pytest.mark.integration` so it can be excluded from unit-only runs.

3. **Python 3.11 patch version**
   - What we know: DO supports 3.11.0–3.11.14 on Ubuntu-22. The user said "unpinned" (let platform pick latest patch).
   - What's unclear: Whether "unpinned" means specify `python-3.11` (minor only) or `python-3.11.14` (latest patch).
   - Recommendation: Use `python-3.11.14` (the latest available 3.11 patch as of research date). DO's runtime.txt format requires MAJOR.MINOR.PATCH. Check DO docs at deploy time in case a newer 3.11.x patch is available.

---

## Sources

### Primary (HIGH confidence)
- Direct code inspection: `apps/efofx-estimate/app/core/security.py`, `app/services/rcf_engine.py`, `app/services/llm_service.py`, `app/core/constants.py`, `requirements.txt`, `pyproject.toml`, `.do/app.yaml`
- PyJWT 2.11.0 docs (pyjwt.readthedocs.io/en/latest/) — encode/decode API, error types
- pwdlib 0.3.0 (github.com/frankie567/pwdlib) — hash/verify API, Python >=3.10 requirement
- OpenAI Python SDK 2.24.0 (pypi.org/project/openai/) — version, Python >=3.9 requirement
- DigitalOcean App Platform Python Buildpack docs (docs.digitalocean.com/products/app-platform/reference/buildpacks/python/) — runtime.txt format, supported Python versions including 3.11.0–3.11.14

### Secondary (MEDIUM confidence)
- OpenAI Structured Outputs cookbook (developers.openai.com/cookbook/examples/structured_outputs_intro) — `beta.chat.completions.parse()` pattern with Pydantic
- PyJWT migration notes (github.com/jpadilla/pyjwt/issues/942) — drop-in compatibility for HS256

### Tertiary (LOW confidence)
- python-jose CVE-2025-61152 (vulert.com) — alg=none bypass; single source, flagged for validation against NVD
- OpenAI v2.0.0 breaking change (github.com/openai/openai-python/releases/tag/v2.0.0) — only one breaking change documented (output field type), main API stable

---

## Metadata

**Confidence breakdown:**
- Bug identification (PRQT-02, PRQT-03): HIGH — confirmed by direct code inspection of actual source files
- Library migration paths (PRQT-01): HIGH — verified against PyPI, official docs
- OpenAI structured output pattern (PRQT-04): HIGH — verified against official cookbook
- DO runtime.txt approach (PRQT-05): HIGH — verified against official DO buildpack docs
- Schema design for estimate output: MEDIUM — Claude has discretion; minimal schema recommended

**Research date:** 2026-02-26
**Valid until:** 2026-05-26 (stable libraries; DO runtime support unlikely to change)
