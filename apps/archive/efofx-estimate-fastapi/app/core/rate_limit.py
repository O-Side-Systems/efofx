"""Rate limiting with slowapi and Valkey backend.

Provides per-tenant tier-based rate limiting and IP-based brute-force
protection for auth endpoints. Backed by Valkey (Redis-compatible) for
distributed state in production; falls back to in-memory in dev.
"""

from slowapi import Limiter
from slowapi.errors import RateLimitExceeded
from slowapi.util import get_remote_address
from fastapi import Request
from fastapi.responses import JSONResponse

from app.core.config import settings

# ---------------------------------------------------------------------------
# Tier-based rate limits
# ---------------------------------------------------------------------------

TIER_LIMITS = {
    "trial": "20/minute",
    "paid": "100/minute",
}

# Default limit for unauthenticated or unresolvable-tenant endpoints
DEFAULT_LIMIT = "30/minute"


# ---------------------------------------------------------------------------
# Key functions
# ---------------------------------------------------------------------------


def get_tenant_id_for_limit(request: Request) -> str:
    """Key function for per-tenant rate limiting.

    Extracts tenant_id from request.state (set by JWT middleware / dependency).
    Falls back to IP-based limiting for unauthenticated requests.
    """
    tenant_id = getattr(request.state, "tenant_id", None)
    if tenant_id:
        return f"tenant:{tenant_id}"
    return f"ip:{get_remote_address(request)}"


def get_tier_limit(request: Request) -> str:
    """Return rate limit string based on tenant tier from request.state.

    Used as a dynamic limit callable with @limiter.limit(get_tier_limit).
    Falls back to trial limits if no tier is set.
    """
    tier = getattr(request.state, "tier", "trial")
    return TIER_LIMITS.get(tier, TIER_LIMITS["trial"])


# ---------------------------------------------------------------------------
# Limiter instance
# ---------------------------------------------------------------------------

# Create limiter with Valkey backend.
# Uses VALKEY_URL from settings. In dev: redis://localhost:6379 (plain).
# In production: valkeys://user:pass@host:port (SSL).
# Note: slowapi/limits library uses "redis" scheme even for Valkey (same protocol).
# If Valkey is unreachable in dev, slowapi falls back to in-memory storage.
# IMPORTANT: In production, VALKEY_URL must use rediss:// scheme (not valkeys://)
# for slowapi/limits library compatibility. valkey.asyncio (used by ValkeyCache)
# also accepts rediss://. See 06-RESEARCH.md Pitfall 1 for details.
limiter = Limiter(
    key_func=get_tenant_id_for_limit,
    default_limits=[DEFAULT_LIMIT],
    storage_uri=settings.VALKEY_URL,
    enabled=settings.RATE_LIMIT_ENABLED,
    # headers_enabled=False (default): FastAPI endpoints return Pydantic models,
    # not Response objects, so slowapi can't inject headers via the decorator.
    # Rate limit headers are injected by the RateLimitHeaderMiddleware in main.py.
)


# ---------------------------------------------------------------------------
# Custom 429 handler
# ---------------------------------------------------------------------------


async def rate_limit_exceeded_handler(
    request: Request, exc: RateLimitExceeded
) -> JSONResponse:
    """Custom 429 handler returning standardized JSON body.

    Response format:
      {"error": "rate_limit_exceeded", "message": "...", "retry_after": N}

    Also sets Retry-After header. slowapi automatically adds
    X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset to
    non-429 responses when the limiter is active.
    """
    # Extract retry_after from the exception detail string.
    # slowapi format: "Rate limit exceeded: X per Y"
    retry_after = 60  # default fallback

    # Try to extract a more accurate retry_after from exc if available
    detail = str(exc.detail) if hasattr(exc, "detail") else "Rate limit exceeded"

    response = JSONResponse(
        status_code=429,
        content={
            "error": "rate_limit_exceeded",
            "message": f"Rate limit exceeded. {detail}",
            "retry_after": retry_after,
        },
        headers={
            "Retry-After": str(retry_after),
        },
    )
    return response
