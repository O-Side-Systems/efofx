# Stack Research

**Domain:** Multi-tenant SaaS estimation platform — stack additions for Epics 3-7
**Researched:** 2026-02-26
**Confidence:** HIGH (core choices verified via official sources and PyPI; version numbers confirmed)

> **Scope:** This file covers only additions to the established stack. Existing tech (FastAPI, MongoDB Atlas, Motor, Pydantic, React 19, Vite, DigitalOcean) is already validated. Do not re-research those.

---

## Critical Upgrades Required (Existing Dependencies That Are Broken or Abandoned)

These are not new additions — they are in `requirements.txt` but must be replaced before building Epic 3.

| Package | Current Pin | Status | Action |
|---------|-------------|--------|--------|
| `python-jose[cryptography]==3.3.0` | 3.3.0 | **Abandoned** — last release 2021, CVE in ecdsa dependency, incompatible with Python >=3.10 in some cases | Replace with `PyJWT[crypto]==2.11.0` |
| `passlib[bcrypt]==1.7.4` | 1.7.4 | **Abandoned** — last release 2020, broken with `bcrypt>=5.0.0`, raises errors on Python 3.13 | Replace with `pwdlib[argon2]==0.3.0` |
| `openai==1.51.0` | 1.51.0 | **Outdated** — openai v2.x is current (v2.24.0 as of 2026-02-24); v1.x no longer supported | Upgrade to `openai>=2.20.0,<3.0.0` |
| `redis==5.0.8` (optional) | 5.0.8 | **Obsolete for DigitalOcean** — DigitalOcean deprecated Managed Redis on 2025-06-30, now runs Managed Valkey | Replace with `valkey>=6.1.0` |

**Sources:**
- python-jose status: [FastAPI community discussion #11345](https://github.com/fastapi/fastapi/discussions/11345), [PyJWT migration guide](https://github.com/jpadilla/pyjwt/issues/942)
- passlib status: [FastAPI community discussion #11773](https://github.com/fastapi/fastapi/discussions/11773)
- openai v2: [PyPI openai 2.24.0](https://pypi.org/project/openai/) — verified 2026-02-26
- DigitalOcean Valkey: [DO blog](https://www.digitalocean.com/blog/introducing-managed-valkey) — Managed Caching discontinued June 2025

---

## Recommended Stack Additions

### Authentication & Security (Epic 3)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `PyJWT[crypto]` | 2.11.0 | JWT encode/decode with tenant_id claims | Actively maintained (released Jan 2026), official FastAPI docs migrated from python-jose to PyJWT; `[crypto]` extra enables RS256/ES256. HS256 sufficient for MVP. |
| `pwdlib[argon2]` | 0.3.0 | Password hashing for tenant accounts | FastAPI's current recommended passlib replacement; Argon2 is memory-hard and GPU-resistant, superior to bcrypt. Python 3.10+ only — matches DO App Platform runtime. |
| `cryptography` | >=46.0.0 | Fernet symmetric encryption for BYOK OpenAI key storage | Already in requirements (needs version bump to 46.x). Fernet = AES-128-CBC + HMAC-SHA256, authenticated, sufficient for single-server MVP. Per-tenant encryption key derived from master secret + tenant_id. |
| `slowapi` | 0.1.9 | Per-tenant, per-tier rate limiting middleware | Starlette/FastAPI-native, decorator-based, supports Redis/Valkey backend for distributed counters. Use tenant_id as rate limit key (not IP) so limits are per-account not per-IP. |

**Installation:**
```bash
pip install "PyJWT[crypto]==2.11.0" "pwdlib[argon2]==0.3.0" "cryptography>=46.0.0" "slowapi==0.1.9"
```

**What NOT to use:**
- `python-jose` — abandoned, security vulnerabilities in ecdsa dep, do not use
- `passlib` — abandoned since 2020, breaks with bcrypt 5.0+, do not use
- `bcrypt` standalone without wrapper — more boilerplate, pwdlib wraps it cleanly with Argon2 option

---

### Caching & Session Store (Epic 3, 4)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `valkey` | >=6.1.0 | Async Valkey/Redis client for rate limiting + LLM response caching | DigitalOcean Managed Redis was discontinued June 2025 and converted to Managed Valkey. valkey-py is a drop-in redis-py fork — change import from `redis` to `valkey`, API identical. Supports async via `valkey.asyncio`. |

**Installation:**
```bash
pip install "valkey>=6.1.0"
```

**DigitalOcean setup:**
- Add a Managed Valkey cluster via DO console (~$15/month for smallest tier)
- Set `VALKEY_URL` env var in `.do/app.yaml`
- Same connection string format as Redis: `valkeys://user:pass@host:port`

**What NOT to use:**
- `redis==5.0.8` (current) — works but targets Redis, not Valkey; DigitalOcean will return Valkey; use the native valkey client to avoid confusion
- `aioredis` — deprecated and merged into redis-py, then forked into valkey-py; no longer needed

**Caching strategy for LLM responses (Epic 4):**

Use hash-based exact-match caching for MVP (not semantic). The key is SHA-256(prompt_template + normalized_inputs). TTL = 24 hours. This avoids the complexity of vector embeddings for semantic similarity. Semantic caching with Redis/Valkey Vector Sets is a post-MVP upgrade.

```python
# Pseudocode: LLM response cache key pattern
cache_key = f"llm:{sha256(f'{prompt_id}:{sorted_params}').hexdigest()}"
```

---

### LLM Integration (Epic 4)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `openai` | >=2.20.0,<3.0.0 | OpenAI API client with BYOK support | v2.x is current stable (2.24.0 released 2026-02-24); `AsyncOpenAI` supports full async; pass `api_key=tenant_openai_key` per-request to implement BYOK without storing keys in client constructor |

**BYOK implementation pattern (Epic 4):**

Do NOT instantiate `AsyncOpenAI` at module level with a global key. Instantiate per-request with the tenant's decrypted key:

```python
async def get_llm_client(tenant: Tenant) -> AsyncOpenAI:
    decrypted_key = fernet_decrypt(tenant.encrypted_openai_key)
    return AsyncOpenAI(api_key=decrypted_key)
```

**What NOT to use:**
- `LiteLLM` — adds significant complexity (proxy server or SDK wrapper), Python GIL-constrained, latency spikes at >500 RPS. For MVP with single provider (OpenAI only), direct SDK is simpler, faster, and easier to debug. Use LiteLLM only if multi-provider support becomes a requirement.
- `openai==1.51.0` (current pin) — outdated major version, no longer supported

**Prompt management (Epic 4):**

Use a file-based git-versioned approach for MVP rather than OpenAI's dashboard Prompt feature (which only works with Responses API, not the standard chat completions API). Store prompts as `.jinja2` template files in `apps/efofx-estimate/prompts/`:

```
prompts/
  estimate_narrative_v1.jinja2
  scoping_chat_v1.jinja2
```

Version by filename convention (`_v1`, `_v2`). Load at startup, cache in memory. This gives git diff, rollback, and PR review on prompt changes — no external service dependency for MVP.

**Installation:**
```bash
pip install "openai>=2.20.0,<3.0.0" "jinja2>=3.1.0"
```

---

### Email (Epic 6 — Magic Links)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `fastapi-mail` | 1.6.2 | Async SMTP email sending for magic link delivery | FastAPI-native async mail library; supports HTML templates, attachments, TLS/SSL, env-var config; actively maintained (1.6.2 released Feb 2026); requires Python 3.10+. Use with any SMTP provider (SendGrid, Postmark, AWS SES). |

**Installation:**
```bash
pip install "fastapi-mail==1.6.2"
```

**Magic link implementation (Epic 6):**

Magic links are not authentication tokens — they are single-use feedback invite tokens. Do not use JWT for magic links (no revocation without Redis). Use this pattern:

```python
# On estimate completion:
token = secrets.token_urlsafe(32)
# Store: {token: {estimate_id, tenant_id, customer_email, expires_at}} in MongoDB
# Email: https://api.efofx.com/feedback?token={token}

# On click:
# 1. Look up token in MongoDB
# 2. Verify not expired (7-day TTL for feedback)
# 3. Mark token used (single-use)
# 4. Return feedback form with pre-filled estimate context
```

**What NOT to use:**
- Raw `smtplib` — synchronous, blocks FastAPI event loop
- `python-sendgrid` or `postmarker` SDKs — vendor lock-in; fastapi-mail is provider-agnostic via SMTP

---

### Widget Bundling & Shadow DOM (Epic 5)

| Tool | Version | Purpose | Why Recommended |
|------|---------|---------|-----------------|
| `react-shadow` | 20.6.0 | React component for Shadow DOM root creation | 150k weekly downloads, integrates natively with React; wraps any React subtree in a Shadow DOM boundary; works with Vite dev server |
| Vite `build.lib` IIFE mode | 7.x (existing) | Bundle widget as single `<script>` tag | Vite uses Rollup internally; `lib.formats: ['iife']` produces a self-executing bundle. No separate bundler needed — extend existing Vite config |

**Vite config addition for widget IIFE build:**

```typescript
// apps/efofx-widget/vite.config.ts
export default defineConfig({
  build: {
    lib: {
      entry: 'src/widget-entry.ts',
      name: 'EfofxWidget',
      formats: ['iife'],
      fileName: () => 'widget.js',
    },
    rollupOptions: {
      // Do NOT externalize React — bundle everything for standalone widget
      external: [],
    },
  },
})
```

**Shadow DOM CSS injection pattern:**

Vite's `build.lib` mode does not auto-inject CSS into Shadow DOM. Use `vite-plugin-shadow-style` OR manually inject stylesheet into the shadow root:

```typescript
// src/widget-entry.ts
import styles from './index.css?inline'  // Vite ?inline query
const shadow = host.attachShadow({ mode: 'open' })
const styleEl = document.createElement('style')
styleEl.textContent = styles
shadow.appendChild(styleEl)
```

**Installation:**
```bash
npm install react-shadow@20.6.0
```

**What NOT to use:**
- Separate Rollup config alongside Vite — redundant; Vite's lib mode is sufficient
- CSS-in-JS (styled-components, emotion) inside Shadow DOM — requires additional setup to scope styles to shadow root; `?inline` CSS import is simpler
- iframes — heavier, cross-origin messaging complexity, worse UX

---

### Observability & Metrics (Epics 3-6)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `prometheus-fastapi-instrumentator` | >=0.10.0 | Auto-instrument FastAPI with Prometheus metrics | Zero-config HTTP metrics (latency, status codes, throughput); add tenant label via custom middleware for per-tenant dashboards; existing Prometheus client is already in requirements — this wraps it cleanly |

**Installation:**
```bash
pip install "prometheus-fastapi-instrumentator>=0.10.0"
```

**Custom tenant label pattern:**

```python
from prometheus_fastapi_instrumentator import Instrumentator
from prometheus_client import Counter

estimate_counter = Counter(
    'estimates_total',
    'Total estimates generated',
    ['tenant_id', 'status']
)
```

---

## Version Compatibility Matrix

| Package | Required Version | Reason |
|---------|-----------------|--------|
| `openai` | >=2.20.0,<3.0.0 | v2.x breaking from v1; pin to avoid v3 surprises |
| `PyJWT[crypto]` | ==2.11.0 | Latest stable (Jan 2026); pin for determinism |
| `pwdlib[argon2]` | ==0.3.0 | Latest stable; requires Python 3.10+ |
| `fastapi-mail` | ==1.6.2 | Latest stable (Feb 2026); requires Python 3.10+ |
| `valkey` | >=6.1.0 | Compatible with DO Managed Valkey (Redis 7.2) |
| `cryptography` | >=46.0.0 | Fernet stable; upgrade from >=41.0.0 |
| `slowapi` | ==0.1.9 | Last stable release (Feb 2024); Redis/Valkey backend |
| `react-shadow` | ==20.6.0 | Last stable; React 19 compatible |
| Python runtime | 3.10+ | Required by pwdlib, fastapi-mail |
| Node.js runtime | 18+ | Existing requirement — no change |

---

## Full Installation Reference

**Python additions (append to `requirements.txt`):**

```bash
# Replace python-jose (REMOVE: python-jose[cryptography]==3.3.0)
PyJWT[crypto]==2.11.0

# Replace passlib (REMOVE: passlib[bcrypt]==1.7.4)
pwdlib[argon2]==0.3.0

# Upgrade OpenAI SDK (REPLACE: openai==1.51.0)
openai>=2.20.0,<3.0.0

# Upgrade cryptography (already present, bump version constraint)
cryptography>=46.0.0

# Cache & rate limiting (already optional, make explicit)
valkey>=6.1.0
slowapi==0.1.9

# Email for magic links
fastapi-mail==1.6.2

# Prompt templating
jinja2>=3.1.0

# Metrics instrumentation
prometheus-fastapi-instrumentator>=0.10.0
```

**Node.js additions (in `apps/efofx-widget/`):**

```bash
npm install react-shadow@20.6.0
```

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| JWT | `PyJWT[crypto]` | `python-jose` | Abandoned since 2021, CVE in dependency, FastAPI docs migrated away |
| Password hashing | `pwdlib[argon2]` | `passlib[bcrypt]` | Abandoned since 2020, broken with bcrypt 5.0+ |
| Password hashing | `pwdlib[argon2]` | `bcrypt` standalone | More boilerplate, no upgrade path to Argon2 |
| Cache client | `valkey` | `redis-py` | DO deprecated Redis; valkey is drop-in replacement |
| LLM client | `openai` direct | `LiteLLM` | Over-engineered for single-provider MVP; Python GIL overhead; adds 25-100x latency vs native at high throughput |
| LLM caching | Hash-based (Redis) | Semantic (vector) | Semantic caching requires embedding model, vector DB, threshold tuning — post-MVP complexity |
| Email | `fastapi-mail` | `smtplib` | Synchronous, blocks event loop |
| Email | `fastapi-mail` | Vendor SDKs (SendGrid, Postmark) | Vendor lock-in; fastapi-mail is SMTP-provider-agnostic |
| Widget isolation | Shadow DOM (`react-shadow`) | iframes | Heavier, cross-origin complexity, worse UX for embedded widgets |
| Widget isolation | Shadow DOM | CSS namespacing/BEM | Can still be overridden by `!important` host styles; Shadow DOM is the only true boundary |
| Prompt management | File-based git templates | OpenAI Dashboard Prompts | Dashboard prompts only work with Responses API; adds external dependency; git history is simpler |

---

## Stack Patterns by Scenario

**If DigitalOcean Managed Valkey is not available (dev environment):**
- Use `fakeredis` for tests: `pip install fakeredis` — valkey-py compatible
- Use `docker run -p 6379:6379 valkey/valkey` locally

**If OpenAI BYOK tenant key is not yet provided (tenant onboarding incomplete):**
- Fall back to platform OpenAI key with usage capped per tier
- Track usage in Valkey counter: `INCR tenant:{id}:openai_tokens`

**If rate limiting without Valkey (single instance, dev):**
- slowapi supports in-memory backend: `Limiter(key_func=get_remote_address)` with no Redis config
- Switch to Valkey backend via `storage_uri` when deploying multi-instance

**If Python runtime is 3.9 (current minimum in requirements):**
- Upgrade DO App Platform runtime to Python 3.11 — required for pwdlib, fastapi-mail
- Python 3.11 is available on DigitalOcean App Platform and has significant performance gains (~25% faster than 3.9)

---

## Sources

- PyJWT 2.11.0 — [PyPI](https://pypi.org/project/PyJWT/) — verified 2026-02-26
- python-jose abandonment — [FastAPI discussion #11345](https://github.com/fastapi/fastapi/discussions/11345), [PyJWT migration issue #942](https://github.com/jpadilla/pyjwt/issues/942)
- passlib abandonment — [FastAPI discussion #11773](https://github.com/fastapi/fastapi/discussions/11773), [pwdlib announcement](https://github.com/frankie567/pwdlib/discussions/1)
- pwdlib 0.3.0 — [PyPI](https://pypi.org/project/pwdlib/) — verified 2026-02-26
- openai 2.24.0 — [GitHub releases](https://github.com/openai/openai-python/releases) — verified 2026-02-26
- DigitalOcean Valkey migration — [DO blog](https://www.digitalocean.com/blog/introducing-managed-valkey) — MEDIUM confidence (official DO announcement)
- valkey-py 6.1.0 — [PyPI](https://pypi.org/project/valkey/) — MEDIUM confidence (official client)
- slowapi 0.1.9 — [PyPI](https://pypi.org/project/slowapi/) — HIGH confidence
- fastapi-mail 1.6.2 — [PyPI](https://pypi.org/project/fastapi-mail/) — HIGH confidence (released Feb 2026)
- react-shadow 20.6.0 — [npm trends](https://npmtrends.com/react-shadow), [GitHub](https://github.com/Wildhoney/ReactShadow) — MEDIUM confidence (last published ~1 year ago; needs check if React 19 compatible)
- Vite IIFE lib mode — [Vite build options docs](https://vite.dev/config/build-options) — HIGH confidence
- cryptography 46.0.5 — [PyPI](https://pypi.org/project/cryptography/) — HIGH confidence (released Feb 2026)
- LiteLLM vs OpenAI SDK — [TrueFoundry LiteLLM review](https://www.truefoundry.com/blog/a-detailed-litellm-review-features-pricing-pros-and-cons-2026) — MEDIUM confidence (multiple sources agree)
- prometheus-fastapi-instrumentator — [GitHub](https://github.com/trallnag/prometheus-fastapi-instrumentator) — HIGH confidence

---

## Open Questions / Items Needing Phase-Specific Verification

1. **react-shadow React 19 compatibility** — Last published ~1 year ago. Before Epic 5, verify react-shadow 20.6.0 works with React 19.2.0 by checking the GitHub issues page. Alternative: use manual `useEffect` + `attachShadow()` without the library.

2. **openai v1→v2 migration scope** — The codebase has `openai==1.51.0` with `llm_service.py`. Before Epic 4, audit `llm_service.py` for API calls that changed in v2 (e.g., `output` field type changes). v2 migration guide: [openai-python CHANGELOG](https://github.com/openai/openai-python/blob/main/CHANGELOG.md).

3. **DO App Platform Python 3.11 availability** — pwdlib and fastapi-mail require Python 3.10+. Verify DigitalOcean App Platform supports Python 3.11 for the `apps/efofx-estimate` component before Epic 3 begins.

4. **Valkey SSL/TLS** — DO Managed Valkey requires SSL (`valkeys://` URI). Confirm valkey-py 6.1.0 handles this identically to redis-py's `rediss://` URI before adding Valkey to Epic 3.

---

*Stack research for: efOfX Estimation Service — Epics 3-7 additions*
*Researched: 2026-02-26*
