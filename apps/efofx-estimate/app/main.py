"""
FastAPI entry point for efOfX Estimation Service.

This module initializes the FastAPI application with all necessary middleware,
routers, and configuration for the estimation service.
"""

from fastapi import FastAPI, Request, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.middleware.trustedhost import TrustedHostMiddleware
from slowapi.errors import RateLimitExceeded
from contextlib import asynccontextmanager
import time
import logging
import os

from app.core.config import settings
from app.core.rate_limit import limiter, rate_limit_exceeded_handler
from app.api.routes import api_router
from app.api.auth import router as auth_router
from app.db.mongodb import connect_to_mongo, close_mongo_connection, health_check as db_health_check

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Application lifespan events."""
    # Startup
    logger.info("Starting efOfX Estimation Service...")
    try:
        await connect_to_mongo()
        logger.info("MongoDB connection established")
    except Exception as e:
        logger.error(f"Failed to connect to MongoDB: {e}")

    yield

    # Shutdown
    logger.info("Shutting down efOfX Estimation Service...")
    await close_mongo_connection()
    logger.info("MongoDB connection closed")


# Create FastAPI application
app = FastAPI(
    title="efOfX Estimation Service",
    description="Natural language-driven project estimation using Reference Class Forecasting (RCF)",
    version="1.0.0",
    docs_url="/docs" if settings.DEBUG else None,
    redoc_url="/redoc" if settings.DEBUG else None,
    lifespan=lifespan,
)

# Register rate limiter
app.state.limiter = limiter
app.add_exception_handler(RateLimitExceeded, rate_limit_exceeded_handler)

# Add middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.ALLOWED_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.add_middleware(
    TrustedHostMiddleware,
    allowed_hosts=settings.ALLOWED_HOSTS
)

# Request timing middleware
@app.middleware("http")
async def add_process_time_header(request: Request, call_next):
    """Add processing time to response headers."""
    start_time = time.time()
    response = await call_next(request)
    process_time = time.time() - start_time
    response.headers["X-Process-Time"] = str(process_time)
    return response

# Include API routes
app.include_router(api_router, prefix="/api/v1")
app.include_router(auth_router, prefix="/api/v1")

@app.get("/health")
async def health_check():
    """
    Health check endpoint with database status.

    Returns:
        dict: Service health status including database connectivity
    """
    db_status = "connected" if await db_health_check() else "disconnected"

    return {
        "status": "healthy" if db_status == "connected" else "degraded",
        "service": "efOfX Estimation Service",
        "database": db_status,
        "version": "1.0.0"
    }

@app.get("/")
async def root():
    """Root endpoint with service information."""
    return {
        "service": "efOfX Estimation Service",
        "version": "1.0.0",
        "description": "Natural language-driven project estimation using RCF"
    }

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(
        "app.main:app",
        host="0.0.0.0",
        port=8000,
        reload=settings.DEBUG
    )
