# Stack Research

**Domain:** Multi-tenant SaaS estimation platform — v1.1 additions only
**Researched:** 2026-02-28
**Confidence:** HIGH (versions verified via PyPI, npm, official DO docs; codebase audited)

> **Scope:** v1.1 Feedback & Quality milestone only. Existing stack (FastAPI, MongoDB Atlas, Motor, Pydantic, React 19, Vite, DigitalOcean, PyJWT, pwdlib, openai v2, slowapi, cryptography) is validated and shipped in v1.0. This file covers only what is NEW or CHANGED for v1.1.

---

## Actual State After Codebase Audit

Before recommending anything new, auditing what's already declared vs what's missing:

| Library | `requirements.txt` | `pyproject.toml` | Actually installed | Status |
|---------|-------------------|------------------|--------------------|--------|
| `fastapi-mail==1.6.2` | MISSING | Present | Unknown | Already declared in pyproject.toml; missing from requirements.txt |
| `valkey>=6.1.0` | MISSING | Present | Not in .venv | Already declared in pyproject.toml; missing from requirements.txt |
| `fakeredis>=2.0.0` | MISSING | Present (dev) | Unknown | In pyproject.toml dev extras; missing from requirements.txt |
| `recharts` | N/A | N/A | N/A | Not added to any app yet — needed for calibration dashboard |

**Key insight:** `pyproject.toml` is the authoritative source and is ahead of `requirements.txt`. The v1.1 dependency work is largely about:
1. Closing the `requirements.txt` / `pyproject.toml` sync gap (tech debt)
2. Wiring the existing `valkey` client into `llm_service.py` (code change, no new dep)
3. Adding `recharts` to a new calibration dashboard app (new frontend only)
4. Setting up workspace config for shared library extraction (infrastructure, no new dep)

---

## What v1.1 Actually Needs

| Feature | Gap Type | Resolution |
|---------|----------|------------|
| Email magic links (feedback) | `fastapi-mail` in pyproject.toml, missing from requirements.txt; pattern from `auth_service.py` needs a `send_feedback_invite_email()` function | Sync requirements.txt; write the new service method |
| Distributed LLM cache | `valkey` in pyproject.toml, missing from requirements.txt; `_response_cache: dict` in `llm_service.py` is per-process | Sync requirements.txt; replace in-memory dict with `valkey.asyncio` client |
| Calibration dashboard | No React dashboard UI exists anywhere | New `apps/efofx-dashboard/` Vite app with recharts |
| Shared library extraction | No workspace config; `TenantAwareCollection`, JWT helpers, Fernet utils are embedded in `efofx-estimate` | uv workspace root + `packages/efofx-core/` |
| Feedback token storage | MongoDB `feedback_tokens` collection needs TTL index on `expires_at` | One-time `create_index()` call in startup; no new library |

---

## New Libraries Required

### Python Backend (truly new — not yet in pyproject.toml)

None. All required Python libraries are already declared in `pyproject.toml`. The work is:
1. Sync `requirements.txt` to match pyproject.toml (see Installation section)
2. Wire the declared libraries into code (Valkey cache, magic link email)

### React Frontend (new Vite app: `apps/efofx-dashboard/`)

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| `recharts` | ^3.7.0 | Line/bar/scatter charts for calibration metrics | recharts 3.7.0 (Jan 2025) explicitly supports React ^16.8.0 \|\| ^17.0.0 \|\| ^18.0.0 \|\| ^19.0.0 as peerDependencies. Lightweight SVG-based (D3 under the hood), 3.6M weekly downloads, works with Tailwind CSS classes directly on SVG elements. No vendor dependency. |
| `@tanstack/react-query` | ^5.0.0 | Server state for dashboard API calls | Calibration metrics, feedback lists, and session data are all async fetches that need loading/error/refresh handling. React Query v5 handles this without Redux boilerplate. Supports React 19. ~6M weekly downloads. |

**What NOT to add for the dashboard:**
- `tremor` — Tremor wraps Recharts in opinionated components. Use Recharts directly; it's simpler and aligns with existing Tailwind usage.
- `Chart.js` / `react-chartjs-2` — Canvas-based (not SVG). Harder to style with Tailwind. No advantage over Recharts for this use case.
- `shadcn/ui` — Adds component scaffolding complexity for what is an internal contractor tool. Plain Tailwind + Recharts is sufficient.
- `D3` directly — Too low-level. Recharts is the right abstraction for a calibration dashboard with standard chart types.
- Redux or Zustand — React Query covers server state. Local UI state is minimal and component-local.

**Installation (new `apps/efofx-dashboard/`):**
```bash
npm create vite@latest efofx-dashboard -- --template react-ts
cd apps/efofx-dashboard
npm install recharts @tanstack/react-query
# tailwindcss + postcss shared from workspace config
npm install -D tailwindcss @tailwindcss/postcss postcss
```

---

## Valkey Integration Pattern

### Problem
`llm_service.py` line 47: `_response_cache: dict[str, str] = {}  # In-memory; upgrade to Valkey for multi-worker`

This is a per-process dict. Multiple Gunicorn workers = no shared cache = cache miss on every non-first-worker request.

### Solution: `valkey.asyncio` connection pool

The `valkey` package is already declared in `pyproject.toml` (`valkey>=6.1.0`). `VALKEY_URL` is already in `settings` and used by slowapi. No new package needed — this is a code wiring task.

**Shared connection pool (new file `app/core/valkey_client.py`):**
```python
import valkey.asyncio as valkey_async
from app.core.config import settings

_pool: valkey_async.ConnectionPool | None = None

def get_valkey_pool() -> valkey_async.ConnectionPool:
    global _pool
    if _pool is None:
        _pool = valkey_async.ConnectionPool.from_url(
            settings.VALKEY_URL,
            max_connections=20,
            decode_responses=True,
        )
    return _pool

async def get_valkey_client() -> valkey_async.Valkey:
    """FastAPI dependency — returns a Valkey client backed by the shared pool."""
    return valkey_async.Valkey(connection_pool=get_valkey_pool())
```

**Cache key (no change from current pattern):**
```python
cache_key = f"llm:{sha256(json.dumps(messages, sort_keys=True)).hexdigest()}"
cached = await client.get(cache_key)
if cached:
    return cached
# ... call OpenAI ...
await client.set(cache_key, response, ex=86400)  # 24h TTL
```

**DigitalOcean Managed Valkey:**
- DO replaced Managed Redis with Managed Valkey in June 2025 (official announcement)
- Smallest tier: 1GB RAM, ~$15/month
- Connection string: `valkeys://doadmin:{password}@{host}:{port}` (note `valkeys://` for TLS)
- The `limits` library used by slowapi accepts `redis://` scheme for Valkey — the existing `VALKEY_URL` setting works for both slowapi and the new LLM cache client
- Inject `VALKEY_URL` as env var in `.do/app.yaml`

**Dev without Valkey:**
```bash
# Docker — matches production Valkey 8.0
docker run -d -p 6379:6379 valkey/valkey:8.0

# Unit tests — fakeredis is in pyproject.toml dev extras
# fakeredis works as a drop-in for valkey.asyncio.Valkey
```

---

## Magic Link Token Pattern

Magic link tokens for feedback invites use the stdlib `secrets` module already imported in `auth_service.py`. No new library needed.

**Why not itsdangerous:** itsdangerous `URLSafeTimedSerializer` creates self-verifying tokens (data + signature, no DB lookup). This project already has MongoDB for every auth check. Storing the feedback token in MongoDB gives: single-use invalidation (mark `used: True`), audit trail, admin visibility, and no secret key rotation risk. The stdlib `secrets.token_urlsafe(32)` (256-bit entropy) is cryptographically secure for this purpose.

**Token storage pattern:**
```python
# New: services/feedback_service.py — create_feedback_token()
import secrets
from datetime import datetime, timedelta, timezone

token = secrets.token_urlsafe(32)      # stdlib — no new dep
expires_at = datetime.now(timezone.utc) + timedelta(days=7)

await db["feedback_tokens"].insert_one({
    "token": token,
    "estimate_id": estimate_id,
    "tenant_id": tenant_id,
    "customer_email": customer_email,
    "expires_at": expires_at,
    "used": False,
})
```

**MongoDB TTL index (one-time startup call):**
```python
# app/db/mongodb.py or app/main.py startup handler
await db["feedback_tokens"].create_index(
    "expires_at",
    expireAfterSeconds=0  # MongoDB purges at expires_at
)
```

**Email delivery:** reuse `fastapi-mail` pattern from `auth_service.py::send_verification_email()`. Same SMTP config (`SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_SERVER`, `SMTP_PORT`, `SMTP_FROM`). No new env vars.

---

## Shared Library Extraction

### Python: uv Workspace

The goal: extract `TenantAwareCollection`, JWT helpers, and Fernet/HKDF utils into `packages/efofx-core/` so a second vertical (IT/dev) can import them without copying code.

`pyproject.toml` uses `setuptools` as build backend. uv workspaces work with any PEP 517 backend.

**Workspace structure:**
```
efofx-workspace/                   ← git root
  pyproject.toml                   ← new: workspace root
  apps/
    efofx-estimate/
      pyproject.toml               ← existing: becomes workspace member
  packages/
    efofx-core/                    ← new: shared utilities
      pyproject.toml
      src/efofx_core/
        db/                        ← TenantAwareCollection
        auth/                      ← JWT helpers
        crypto/                    ← Fernet/HKDF
        models/                    ← base Pydantic models
```

**New root `pyproject.toml`:**
```toml
[tool.uv.workspace]
members = ["apps/efofx-estimate", "packages/efofx-core"]
```

**Add to `apps/efofx-estimate/pyproject.toml` dependencies:**
```toml
dependencies = [
  # ... existing deps ...
  "efofx-core",
]

[tool.uv.sources]
efofx-core = { workspace = true }
```

**Important scope constraint:** Extract only what a second vertical ACTUALLY needs in v1.1. Do not extract the entire service. Start with:
- `TenantAwareCollection` wrapper class
- JWT token encode/decode helpers
- Fernet key derivation (HKDF pattern)

Defer extracting API models, RCF engine, estimation logic — those are domain-specific.

### TypeScript: npm Workspaces

**Workspace structure:**
```
efofx-workspace/
  package.json                     ← new: workspace root
  apps/
    efofx-widget/package.json      ← existing
    efofx-dashboard/package.json   ← new
  packages/
    efofx-ui/package.json          ← new: shared types + components
      src/
        types/                     ← API response TypeScript interfaces
        components/                ← Button, LoadingSpinner, ErrorBanner
```

**Root `package.json`:**
```json
{
  "name": "efofx-workspace",
  "private": true,
  "workspaces": ["apps/*", "packages/*"]
}
```

**Vite alias in each app's `vite.config.ts`:**
```typescript
import path from 'path'
resolve: {
  alias: {
    '@efofx/ui': path.resolve(__dirname, '../../packages/efofx-ui/src')
  }
}
```

**Scope constraint for v1.1:** Extract TypeScript API response interfaces and 2-3 shared UI primitives. Do not build a full component library — that is post-v1.1 scope when the second vertical exists to validate what's actually shared.

---

## Version Compatibility Matrix (v1.1)

| Package | Version | Constraint Reason |
|---------|---------|-----------------|
| `fastapi-mail` | ==1.6.2 | Latest stable (Feb 17, 2026); already in pyproject.toml; sync to requirements.txt |
| `valkey` | >=6.1.0,<7.0.0 | 6.1.1 current (Aug 2025); upper bound prevents v7 breaking API changes; already in pyproject.toml |
| `fakeredis` | >=2.0.0 | Dev/test only; compatible with valkey.asyncio; already in pyproject.toml dev extras |
| `recharts` | ^3.7.0 | 3.7.0 (Jan 2025); peerDependencies confirmed: React ^16\|\|^17\|\|^18\|\|^19; new dashboard dep |
| `@tanstack/react-query` | ^5.0.0 | v5 is current LTS; React 19 compatible; new dashboard dep |
| Python runtime | >=3.11 | pyproject.toml specifies requires-python = ">=3.11"; fastapi-mail requires 3.10+ |
| Node.js runtime | 18+ | No change from v1.0 |
| Valkey server | 8.0 | DO Managed Valkey runs Valkey 8.0 (Redis 7.2.4 protocol compatible) |

---

## Installation Summary

**Python — sync `requirements.txt` with `pyproject.toml`:**
```
# These are declared in pyproject.toml but missing from requirements.txt:

# Email (magic links — already lazy-imported in auth_service.py)
fastapi-mail==1.6.2

# Distributed cache (replaces per-process dict in llm_service.py)
valkey>=6.1.0,<7.0.0

# Dev-only (add to requirements-dev.txt or keep in pyproject.toml extras)
fakeredis>=2.0.0
```

**Node.js — `apps/efofx-dashboard/` (new app):**
```bash
npm install recharts @tanstack/react-query
npm install -D tailwindcss @tailwindcss/postcss postcss
```

**Python workspace — new root `pyproject.toml`:**
```toml
[tool.uv.workspace]
members = ["apps/efofx-estimate", "packages/efofx-core"]
```

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Magic link tokens | `secrets.token_urlsafe` + MongoDB | `itsdangerous` URLSafeTimedSerializer | MongoDB lookup is already required for single-use invalidation; signed tokens provide no benefit when you're already hitting the DB |
| Valkey client | `valkey` (valkey-py) | `valkey-glide` | GLIDE is Rust-core, higher throughput, but heavier install; overkill for 15-tenant MVP; revisit at 500+ tenants |
| Valkey client | `valkey` (valkey-py) | `redis-py` | DO retired Managed Redis in June 2025; use the native Valkey client to avoid scheme/version confusion |
| Dashboard charts | `recharts` | `tremor` | Tremor wraps Recharts in opinionated components; conflicts with existing Tailwind conventions; adds abstraction for no gain |
| Dashboard charts | `recharts` | `Chart.js` | Canvas-based, not SVG; harder to style with Tailwind; worse React integration |
| Dashboard charts | `recharts` | `D3` directly | Too low-level for standard calibration chart types; recharts is the right abstraction |
| Python shared lib | uv workspace | pip editable install | uv workspace is native monorepo support; `pip install -e` is legacy approach |
| Python shared lib | uv workspace | Poetry workspaces | Project uses setuptools + uv; no toolchain switch needed |
| TS shared lib | npm workspaces | pnpm workspaces | Project uses npm (package-lock.json exists); don't switch package managers mid-project |
| Email | `fastapi-mail` (existing) | SendGrid/Postmark SDK | Vendor lock-in; fastapi-mail is SMTP-provider-agnostic via env vars |

---

## Open Questions / Phase Verification Required

1. **`requirements.txt` vs `pyproject.toml` authority** — The project has both. Verify which one the DigitalOcean App Platform build uses. If DO uses `pyproject.toml` with `pip install -e .`, then `requirements.txt` is just a dev convenience and the sync gap is less critical. If DO uses `pip install -r requirements.txt`, then the gap causes missing deps in production.

2. **slowapi VALKEY_URL scheme** — The `limits` library (slowapi backend) may require `redis://` scheme even for a Valkey host. DO Managed Valkey requires TLS, so the URL is `valkeys://...`. Verify this works with slowapi before Phase 1. If not, use `redis://...?ssl=true` workaround.

3. **fakeredis valkey-py compatibility** — `fakeredis` was originally written for redis-py. Verify `fakeredis.aioredis.FakeRedis()` is a drop-in for `valkey.asyncio.Valkey` in unit tests. Confirmed in pyproject.toml dev extras, but needs functional verification.

4. **uv workspace with existing setuptools pyproject.toml** — The existing `apps/efofx-estimate/pyproject.toml` uses `setuptools` build backend. Confirm uv workspace membership works with setuptools (it should — uv workspaces are PEP 517 build-backend agnostic).

5. **recharts React 19 actual install** — peerDependency range confirms React 19 support in recharts 3.7.0. Still run `npm install` in the new dashboard app to confirm no `--legacy-peer-deps` flag is needed (some transitive deps may still declare React 18 peer requirements).

---

## Sources

- fastapi-mail 1.6.2 — [PyPI](https://pypi.org/project/fastapi-mail/) — verified 2026-02-28 (released Feb 17, 2026) — HIGH confidence
- valkey-py 6.1.1 — [PyPI](https://pypi.org/project/valkey/) — verified 2026-02-28 — HIGH confidence
- valkey async connection pool pattern — [valkey-py docs](https://valkey-py.readthedocs.io/en/latest/examples/asyncio_examples.html) — HIGH confidence
- DigitalOcean Managed Valkey — [DO blog](https://www.digitalocean.com/blog/introducing-managed-valkey), [DO product page](https://www.digitalocean.com/products/managed-databases-valkey) — HIGH confidence (official DO announcement)
- recharts 3.7.0 peerDependencies — [recharts GitHub package.json](https://github.com/recharts/recharts/blob/main/package.json) — HIGH confidence (React 19 confirmed in peerDep range)
- recharts latest version (3.7.0) — WebSearch confirmed Jan 2025 release — HIGH confidence
- @tanstack/react-query v5 — [TanStack docs](https://tanstack.com/query/v5) — HIGH confidence
- uv workspaces — [uv docs](https://docs.astral.sh/uv/concepts/workspaces/) — HIGH confidence
- itsdangerous 2.2.0 — [PyPI](https://pypi.org/project/itsdangerous/), [docs](https://itsdangerous.palletsprojects.com/) — HIGH confidence (evaluated and NOT recommended for this use case)
- secrets.token_urlsafe — [Python stdlib](https://docs.python.org/3/library/secrets.html) — HIGH confidence
- slowapi Redis/Valkey backend — [slowapi docs](https://slowapi.readthedocs.io/en/latest/) — HIGH confidence
- pyproject.toml audit — direct file read of `apps/efofx-estimate/pyproject.toml` — HIGH confidence

---

*Stack research for: efOfX Estimation Service — v1.1 Feedback & Quality additions*
*Researched: 2026-02-28*
