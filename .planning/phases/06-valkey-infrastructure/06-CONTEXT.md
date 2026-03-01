# Phase 6: Valkey Infrastructure - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace per-process LLM dict cache with distributed Valkey cache. Cross-worker cache hits work correctly, cache keys are tenant-scoped, and Valkey outages fall back to live LLM calls without crashing the service. Provisioning the DigitalOcean Managed Valkey instance is included.

</domain>

<decisions>
## Implementation Decisions

### Cache lifetime & scope
- 24-hour TTL on all cached entries
- Cache both the raw LLM response and the parsed/structured estimation result
- TTL-only expiration — no version-tagged invalidation, no prompt-version cache busting
- No manual flush endpoint or CLI command — 24h TTL handles staleness

### Fallback behavior
- When Valkey is unreachable, fall back to direct LLM calls with no caching (no per-process dict fallback)
- Log a warning on first Valkey failure, suppress repeated warnings for a cooldown period (avoid log flood)
- Lazy reconnect — each incoming request tries Valkey; if it's back, cache resumes. No background health check
- Service starts in degraded mode if Valkey is unavailable at startup — logs warning, serves via live LLM

### Valkey provisioning
- DigitalOcean Managed Valkey, smallest available instance (1 GB)
- Same region as the application servers
- No persistence (ephemeral) — cache-only use case, data loss on restart is acceptable
- Connection string via VALKEY_URL environment variable

### Cache key granularity
- Key composition: tenant_id + hash of estimation input parameters
- Namespaced keys: `efofx:llm:{tenant_id}:{input_hash}`
- JSON serialization for cached values (human-readable, debuggable via Valkey CLI)
- Logging only for observability — cache hits/misses at debug level, no structured metrics

### Claude's Discretion
- Valkey client library choice (valkey-py, redis-py compatibility, etc.)
- Connection pooling configuration
- Exact hash algorithm for input parameters
- Error handling details beyond the fallback policy
- How to structure the ValkeyCache service class

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 06-valkey-infrastructure*
*Context gathered: 2026-02-28*
