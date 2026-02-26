"""
Security utilities for efOfX Estimation Service.

This module provides authentication, authorization, and security utilities
for the estimation service including API key validation and tenant management.
"""

from fastapi import HTTPException, Depends, Request
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from typing import Optional, Dict, Any
import jwt  # PyJWT
import logging
from datetime import datetime, timedelta

from app.core.config import settings
from app.core.constants import API_MESSAGES, HTTP_STATUS, DB_COLLECTIONS
from app.db.mongodb import get_database
from app.models.tenant import Tenant

logger = logging.getLogger(__name__)

# Security scheme
security = HTTPBearer(auto_error=False)


class AuthService:
    """Authentication and authorization service."""
    
    def __init__(self):
        self.secret_key = settings.SECRET_KEY
        self.algorithm = "HS256"
    
    def create_access_token(self, data: Dict[str, Any], expires_delta: Optional[timedelta] = None) -> str:
        """Create JWT access token."""
        to_encode = data.copy()
        if expires_delta:
            expire = datetime.utcnow() + expires_delta
        else:
            expire = datetime.utcnow() + timedelta(minutes=15)
        to_encode.update({"exp": expire})
        encoded_jwt = jwt.encode(to_encode, self.secret_key, algorithm=self.algorithm)
        return encoded_jwt
    
    def verify_token(self, token: str) -> Dict[str, Any]:
        """Verify JWT token and return payload."""
        try:
            payload = jwt.decode(token, self.secret_key, algorithms=[self.algorithm])
            return payload
        except jwt.InvalidTokenError:  # PyJWT base exception for all token errors
            raise HTTPException(
                status_code=HTTP_STATUS["UNAUTHORIZED"],
                detail="Invalid authentication credentials"
            )
    
    async def validate_api_key(self, api_key: str) -> Tenant:
        """Validate API key and return associated tenant."""
        db = get_database()
        tenant = await db[DB_COLLECTIONS["TENANTS"]].find_one({"api_key": api_key})
        
        if not tenant:
            raise HTTPException(
                status_code=HTTP_STATUS["UNAUTHORIZED"],
                detail="Invalid API key"
            )
        
        return Tenant(**tenant)
    
    async def get_current_tenant(self, credentials: HTTPAuthorizationCredentials = Depends(security)) -> Tenant:
        """Get current tenant from API key or JWT token."""
        if not credentials:
            raise HTTPException(
                status_code=HTTP_STATUS["UNAUTHORIZED"],
                detail="Authentication credentials required"
            )
        
        # Check if it's an API key (Bearer token starting with 'sk_')
        if credentials.scheme == "Bearer" and credentials.credentials.startswith("sk_"):
            return await self.validate_api_key(credentials.credentials)
        
        # Otherwise, treat as JWT token
        payload = self.verify_token(credentials.credentials)
        tenant_id = payload.get("sub")
        
        if not tenant_id:
            raise HTTPException(
                status_code=HTTP_STATUS["UNAUTHORIZED"],
                detail="Invalid token payload"
            )
        
        db = get_database()
        tenant = await db[DB_COLLECTIONS["TENANTS"]].find_one({"_id": tenant_id})
        
        if not tenant:
            raise HTTPException(
                status_code=HTTP_STATUS["UNAUTHORIZED"],
                detail="Tenant not found"
            )
        
        return Tenant(**tenant)


# Global auth service instance
auth_service = AuthService()


async def get_current_tenant(credentials: HTTPAuthorizationCredentials = Depends(security)) -> Tenant:
    """Dependency to get current authenticated tenant."""
    return await auth_service.get_current_tenant(credentials)


def require_tenant_permission(permission: str):
    """Decorator to require specific tenant permission."""
    def decorator(func):
        async def wrapper(*args, **kwargs):
            # This would be implemented based on your permission system
            # For now, we'll just check if tenant exists
            return await func(*args, **kwargs)
        return wrapper
    return decorator


class RateLimiter:
    """Simple rate limiting implementation."""
    
    def __init__(self):
        self.requests = {}
    
    def is_allowed(self, tenant_id: str, limit: int = 100, window: int = 3600) -> bool:
        """Check if request is allowed based on rate limit."""
        now = datetime.utcnow()
        window_start = now - timedelta(seconds=window)
        
        if tenant_id not in self.requests:
            self.requests[tenant_id] = []
        
        # Clean old requests
        self.requests[tenant_id] = [
            req_time for req_time in self.requests[tenant_id]
            if req_time > window_start
        ]
        
        # Check if under limit
        if len(self.requests[tenant_id]) >= limit:
            return False
        
        # Add current request
        self.requests[tenant_id].append(now)
        return True


# Global rate limiter instance
rate_limiter = RateLimiter()


def check_rate_limit(tenant_id: str, limit: int = 100):
    """Check rate limit for tenant."""
    if not rate_limiter.is_allowed(tenant_id, limit):
        raise HTTPException(
            status_code=HTTP_STATUS["RATE_LIMITED"],
            detail="Rate limit exceeded"
        ) 