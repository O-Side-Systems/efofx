"""
API layer for efOfX Estimation Service.

This package contains all API route handlers and endpoints
for the estimation service.
"""

from .routes import api_router

__all__ = [
    "api_router",
]
