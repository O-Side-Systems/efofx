"""
API routes for efOfX Estimation Service.

This module defines all API endpoints and route handlers
for the estimation service.
"""

from fastapi import APIRouter, Depends, HTTPException, Request, UploadFile, File
from typing import List, Optional
import logging

from app.core.rate_limit import limiter, get_tenant_id_for_limit, get_tier_limit
from app.core.security import get_current_tenant
from app.core.constants import API_MESSAGES, HTTP_STATUS
from app.models.tenant import Tenant
from app.models.estimation import EstimationRequest, EstimationResponse
from app.models.chat import ChatRequest, ChatResponse
from app.models.feedback import FeedbackCreate, FeedbackSummary
from app.services.estimation_service import EstimationService
from app.services.chat_service import ChatService
from app.services.feedback_service import FeedbackService

logger = logging.getLogger(__name__)

# Create main API router
api_router = APIRouter()

# Service dependency injection (lazy initialization)
def get_estimation_service() -> EstimationService:
    """Get estimation service instance."""
    return EstimationService()

def get_chat_service() -> ChatService:
    """Get chat service instance."""
    return ChatService()

def get_feedback_service() -> FeedbackService:
    """Get feedback service instance."""
    return FeedbackService()


# Estimation endpoints
@api_router.post("/estimate/start", response_model=EstimationResponse)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def start_estimation(
    http_request: Request,
    request: EstimationRequest,
    tenant: Tenant = Depends(get_current_tenant),
    estimation_service: EstimationService = Depends(get_estimation_service)
):
    """Start a new estimation session."""
    try:
        response = await estimation_service.start_estimation(request, tenant)
        return response
    except Exception as e:
        logger.error(f"Error starting estimation: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["INTERNAL_ERROR"],
            detail=API_MESSAGES["DB_ERROR"]
        )


@api_router.get("/estimate/{session_id}", response_model=EstimationResponse)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def get_estimation(
    request: Request,
    session_id: str,
    tenant: Tenant = Depends(get_current_tenant),
    estimation_service: EstimationService = Depends(get_estimation_service)
):
    """Get estimation session status and results."""
    try:
        response = await estimation_service.get_estimation(session_id, tenant)
        return response
    except Exception as e:
        logger.error(f"Error getting estimation: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["NOT_FOUND"],
            detail=API_MESSAGES["ESTIMATION_NOT_FOUND"]
        )


@api_router.post("/estimate/{session_id}/upload")
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def upload_image(
    request: Request,
    session_id: str,
    file: UploadFile = File(...),
    tenant: Tenant = Depends(get_current_tenant),
    estimation_service: EstimationService = Depends(get_estimation_service)
):
    """Upload image for estimation session."""
    try:
        result = await estimation_service.upload_image(session_id, file, tenant)
        return {"message": "Image uploaded successfully", "image_url": result}
    except Exception as e:
        logger.error(f"Error uploading image: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["BAD_REQUEST"],
            detail="Invalid image file"
        )


# Chat endpoints
@api_router.post("/chat/send", response_model=ChatResponse)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def send_chat_message(
    http_request: Request,
    request: ChatRequest,
    tenant: Tenant = Depends(get_current_tenant),
    chat_service: ChatService = Depends(get_chat_service)
):
    """Send a chat message for estimation."""
    try:
        response = await chat_service.send_message(request, tenant)
        return response
    except Exception as e:
        logger.error(f"Error sending chat message: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["INTERNAL_ERROR"],
            detail="Failed to process chat message"
        )


@api_router.get("/chat/{session_id}/history")
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def get_chat_history(
    request: Request,
    session_id: str,
    tenant: Tenant = Depends(get_current_tenant),
    chat_service: ChatService = Depends(get_chat_service)
):
    """Get chat history for a session."""
    try:
        history = await chat_service.get_chat_history(session_id, tenant)
        return {"session_id": session_id, "messages": history}
    except Exception as e:
        logger.error(f"Error getting chat history: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["NOT_FOUND"],
            detail="Chat session not found"
        )


# Feedback endpoints
@api_router.post("/feedback/submit")
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def submit_feedback(
    request: Request,
    feedback: FeedbackCreate,
    tenant: Tenant = Depends(get_current_tenant),
    feedback_service: FeedbackService = Depends(get_feedback_service)
):
    """Submit feedback for an estimation."""
    try:
        result = await feedback_service.submit_feedback(feedback, tenant)
        return {"message": "Feedback submitted successfully", "feedback_id": result}
    except Exception as e:
        logger.error(f"Error submitting feedback: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["BAD_REQUEST"],
            detail="Failed to submit feedback"
        )


@api_router.get("/feedback/summary", response_model=FeedbackSummary)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def get_feedback_summary(
    request: Request,
    tenant: Tenant = Depends(get_current_tenant),
    feedback_service: FeedbackService = Depends(get_feedback_service)
):
    """Get feedback summary for tenant."""
    try:
        summary = await feedback_service.get_feedback_summary(tenant)
        return summary
    except Exception as e:
        logger.error(f"Error getting feedback summary: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["INTERNAL_ERROR"],
            detail="Failed to get feedback summary"
        )


# Health and status endpoints
@api_router.get("/status")
async def get_service_status():
    """Get service status and health information."""
    try:
        from app.db.mongodb import health_check, get_database_stats
        
        db_healthy = await health_check()
        db_stats = await get_database_stats()
        
        return {
            "status": "healthy" if db_healthy else "unhealthy",
            "database": {
                "connected": db_healthy,
                "stats": db_stats
            },
            "service": "efOfX Estimation Service",
            "version": "1.0.0"
        }
    except Exception as e:
        logger.error(f"Error getting service status: {e}")
        return {
            "status": "unhealthy",
            "error": str(e)
        }


# Admin endpoints (for internal use)
@api_router.get("/admin/tenants")
async def list_tenants(
    tenant: Tenant = Depends(get_current_tenant),
    limit: int = 10,
    offset: int = 0
):
    """List tenants (admin only)."""
    try:
        # This would include proper admin authorization
        from app.services.tenant_service import TenantService
        tenant_service = TenantService()
        tenants = await tenant_service.list_tenants(limit, offset)
        return {"tenants": tenants, "total": len(tenants)}
    except Exception as e:
        logger.error(f"Error listing tenants: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["FORBIDDEN"],
            detail="Access denied"
        )


@api_router.get("/admin/reference-classes")
async def list_reference_classes(
    tenant: Tenant = Depends(get_current_tenant),
    category: Optional[str] = None
):
    """List reference classes (admin only)."""
    try:
        from app.services.reference_service import ReferenceService
        reference_service = ReferenceService()
        classes = await reference_service.list_reference_classes(category)
        return {"reference_classes": classes}
    except Exception as e:
        logger.error(f"Error listing reference classes: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["FORBIDDEN"],
            detail="Access denied"
        ) 