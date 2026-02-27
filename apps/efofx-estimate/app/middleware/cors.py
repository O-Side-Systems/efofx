"""
Per-tenant CORS middleware for efOfX white-label widget.

Extends Starlette's CORSMiddleware to support dynamic per-tenant origin checking.
Static ALLOWED_ORIGINS (admin dashboard, dev) are checked first.
Per-tenant origins are loaded lazily via a module-level cache populated by
widget_service during the public branding endpoint call.

Cache design: module-level dict avoids async DB calls in middleware and avoids
loading all tenant origins at startup. Cache is populated on first branding
request per tenant and refreshed on every subsequent branding call.
"""

import logging
from typing import Sequence

from starlette.middleware.cors import CORSMiddleware
from starlette.types import ASGIApp

logger = logging.getLogger(__name__)

# Module-level cache: tenant_id -> list of allowed origins.
# Populated by widget_service.get_branding_by_prefix when a tenant is found.
# Shared between the middleware and widget_service via module import.
_tenant_origins_cache: dict[str, list[str]] = {}


class TenantAwareCORSMiddleware(CORSMiddleware):
    """Extends CORSMiddleware with per-tenant dynamic origin checking.

    First checks the static allow_origins list (admin dashboard, dev).
    Then checks tenant-registered domains stored in the module-level
    _tenant_origins_cache dict, which is populated lazily by widget_service
    during branding lookups.

    This avoids async DB calls inside synchronous middleware and avoids
    loading all tenant origins at startup.
    """

    def __init__(self, app: ASGIApp, *, allow_origins: Sequence[str] = (), **kwargs):
        super().__init__(app, allow_origins=allow_origins, **kwargs)

    def is_allowed_origin(self, origin: str) -> bool:
        # Fast path: check static ALLOWED_ORIGINS (admin/dashboard/dev)
        if super().is_allowed_origin(origin):
            return True
        # Check all cached per-tenant origins
        for origins in _tenant_origins_cache.values():
            if origin in origins:
                return True
        return False

    def update_tenant_origins(self, tenant_id: str, origins: list[str]) -> None:
        """Update cached origins for a tenant.

        Provided for explicit updates outside of the branding flow.
        Normally populated by widget_service.get_branding_by_prefix.
        """
        _tenant_origins_cache[tenant_id] = origins
        logger.debug("Updated CORS origins cache for tenant %s: %s", tenant_id, origins)
