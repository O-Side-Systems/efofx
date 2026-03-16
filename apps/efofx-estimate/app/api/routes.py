"""
API routes for efOfX Estimation Service.

This module defines all API endpoints and route handlers
for the estimation service.
"""

from fastapi import APIRouter, Depends, HTTPException, Request, UploadFile, File
from fastapi.responses import StreamingResponse
from typing import Optional
import json
import logging

from openai import OpenAIError

from app.core.rate_limit import limiter, get_tenant_id_for_limit, get_tier_limit
from app.core.security import get_current_tenant
from app.core.constants import API_MESSAGES, HTTP_STATUS
from app.models.tenant import Tenant
from app.models.estimation import EstimationResponse
from app.models.chat import ChatRequest, ChatResponse
from app.models.feedback import FeedbackCreate, FeedbackSummary
from app.services.estimation_service import EstimationService
from app.services.chat_service import ChatService
from app.services.feedback_service import FeedbackService
from app.services.llm_service import LLMService, get_llm_service, classify_openai_error
from app.services.prompt_service import PromptService

logger = logging.getLogger(__name__)

# Create main API router
api_router = APIRouter()


# Service dependency injection — BYOK key flows through get_llm_service
def get_estimation_service(
    llm_service: LLMService = Depends(get_llm_service),
) -> EstimationService:
    """Get estimation service instance with tenant BYOK key injected."""
    return EstimationService(llm_service=llm_service)


def get_chat_service(llm_service: LLMService = Depends(get_llm_service)) -> ChatService:
    """Get chat service instance with tenant BYOK key injected."""
    return ChatService(llm_service=llm_service)


def get_feedback_service() -> FeedbackService:
    """Get feedback service instance."""
    return FeedbackService()


# Estimation endpoints
@api_router.get("/estimate/{session_id}", response_model=EstimationResponse)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def get_estimation(
    request: Request,
    session_id: str,
    tenant: Tenant = Depends(get_current_tenant),
    estimation_service: EstimationService = Depends(get_estimation_service),
):
    """Get estimation session status and results."""
    try:
        response = await estimation_service.get_estimation(session_id, tenant)
        return response
    except Exception as e:
        logger.error(f"Error getting estimation: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["NOT_FOUND"],
            detail=API_MESSAGES["ESTIMATION_NOT_FOUND"],
        )


@api_router.post("/estimate/{session_id}/upload")
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def upload_image(
    request: Request,
    session_id: str,
    file: UploadFile = File(...),
    tenant: Tenant = Depends(get_current_tenant),
    estimation_service: EstimationService = Depends(get_estimation_service),
):
    """Upload image for estimation session."""
    try:
        result = await estimation_service.upload_image(session_id, file, tenant)
        return {"message": "Image uploaded successfully", "image_url": result}
    except Exception as e:
        logger.error(f"Error uploading image: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["BAD_REQUEST"], detail="Invalid image file"
        )


# Chat endpoints
@api_router.post("/chat/send", response_model=ChatResponse)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def send_chat_message(
    http_request: Request,
    request: ChatRequest,
    tenant: Tenant = Depends(get_current_tenant),
    chat_service: ChatService = Depends(get_chat_service),
):
    """Send a chat message for conversational project scoping."""
    try:
        response = await chat_service.send_message(request, tenant)
        return response
    except HTTPException:
        raise  # Let 402 etc. propagate
    except Exception as e:
        logger.error(f"Error sending chat message: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["INTERNAL_ERROR"],
            detail="Failed to process chat message",
        )


@api_router.post("/chat/{session_id}/generate-estimate")
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def generate_estimate_stream(
    request: Request,
    session_id: str,
    tenant: Tenant = Depends(get_current_tenant),
    llm_service: LLMService = Depends(get_llm_service),
):
    """Generate estimate and stream narrative via SSE.

    Flow:
    1. Emit 'thinking' event
    2. Retrieve chat session and validate readiness
    3. Generate structured estimation (non-streaming, via beta.chat.completions.parse)
    4. Emit 'estimate' event with structured data as JSON
    5. Stream narrative token-by-token as 'data:' events
    6. Emit 'done' event with session_id

    On error: emit 'error' event with error_type and close stream.
    """
    chat_service = ChatService(llm_service=llm_service)
    estimation_service = EstimationService(llm_service=llm_service)

    async def event_generator():
        try:
            # Phase 1: Thinking state (NARR-04)
            yield "event: thinking\ndata: {}\n\n"

            # Phase 2: Retrieve chat session
            session = await chat_service.get_session(session_id, tenant)
            if session.status not in ("ready", "active"):
                yield (
                    f'event: error\ndata: {json.dumps({"error_type": "invalid_state", "message": "Session is not ready for estimate generation"})}\n\n'  # noqa: E501
                )
                return

            # Phase 3: Generate structured estimation (non-streaming)
            est_session, estimation_output = (
                await estimation_service.generate_from_chat(
                    session=session,
                    tenant=tenant,
                )
            )

            # Phase 4: Emit structured estimate data
            estimate_json = estimation_output.model_dump_json()
            yield f"event: estimate\ndata: {estimate_json}\n\n"

            # Phase 5: Stream narrative
            narrative_prompt = PromptService.get("narrative", "latest")

            # Format cost breakdown for narrative prompt
            cost_breakdown_str = "\n".join(
                f"- {cat.category}: ${cat.p50_cost:,.0f} - ${cat.p80_cost:,.0f} ({cat.percentage_of_total:.0%} of total)"  # noqa: E501
                for cat in estimation_output.cost_breakdown
            )
            adjustment_str = "\n".join(
                f"- {af.name}: {af.multiplier}x — {af.reason}"
                for af in estimation_output.adjustment_factors
            )
            assumptions_str = "\n".join(f"- {a}" for a in estimation_output.assumptions)

            # Build narrative messages
            user_content = narrative_prompt["user_prompt_template"].format(
                project_description=est_session.description,
                total_cost_p50=estimation_output.total_cost_p50,
                total_cost_p80=estimation_output.total_cost_p80,
                timeline_weeks_p50=estimation_output.timeline_weeks_p50,
                timeline_weeks_p80=estimation_output.timeline_weeks_p80,
                cost_breakdown=cost_breakdown_str,
                adjustment_factors=adjustment_str,
                assumptions=assumptions_str,
            )

            messages = [
                {"role": "system", "content": narrative_prompt["system_prompt"]},
                {"role": "user", "content": user_content},
            ]

            # Stream narrative tokens — None values already filtered in stream_chat_completion  # noqa: E501
            async for token in llm_service.stream_chat_completion(messages):
                # Escape newlines to preserve SSE framing
                escaped = token.replace("\n", "\\n")
                yield f"data: {escaped}\n\n"

            # Phase 6: Mark chat session as completed
            await chat_service.mark_completed(
                session_id, tenant, est_session.session_id
            )

            # Phase 7: Done
            yield f'event: done\ndata: {json.dumps({"session_id": est_session.session_id})}\n\n'  # noqa: E501

        except OpenAIError as exc:
            error_type, status_code = classify_openai_error(exc)
            error_msg = {
                "invalid_key": "Invalid OpenAI API key. Update your key in Settings.",
                "quota_exhausted": "OpenAI quota exhausted. Recharge your OpenAI account.",  # noqa: E501
                "transient": "We're having trouble generating a response. Please try again in a moment.",  # noqa: E501
                "unknown": "An unexpected error occurred during AI processing.",
            }.get(error_type, "An unexpected error occurred.")
            yield f'event: error\ndata: {json.dumps({"error_type": error_type, "message": error_msg, "status": status_code})}\n\n'  # noqa: E501

        except ValueError as exc:
            # Chat session not found or invalid state
            yield f'event: error\ndata: {json.dumps({"error_type": "invalid_session", "message": str(exc)})}\n\n'  # noqa: E501

        except Exception as exc:
            logger.error(f"Unexpected error in estimate stream: {exc}")
            yield f'event: error\ndata: {json.dumps({"error_type": "unknown", "message": "An unexpected error occurred."})}\n\n'  # noqa: E501

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "X-Accel-Buffering": "no",
            "Connection": "keep-alive",
        },
    )


@api_router.get("/chat/{session_id}/history")
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def get_chat_history(
    request: Request,
    session_id: str,
    tenant: Tenant = Depends(get_current_tenant),
    chat_service: ChatService = Depends(get_chat_service),
):
    """Get chat history for a session."""
    try:
        history = await chat_service.get_chat_history(session_id, tenant)
        return {"session_id": session_id, "messages": history}
    except Exception as e:
        logger.error(f"Error getting chat history: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["NOT_FOUND"], detail="Chat session not found"
        )


# Feedback endpoints
@api_router.post("/feedback/submit")
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def submit_feedback(
    request: Request,
    feedback: FeedbackCreate,
    tenant: Tenant = Depends(get_current_tenant),
    feedback_service: FeedbackService = Depends(get_feedback_service),
):
    """Submit feedback for an estimation."""
    try:
        result = await feedback_service.submit_feedback(feedback, tenant)
        return {"message": "Feedback submitted successfully", "feedback_id": result}
    except Exception as e:
        logger.error(f"Error submitting feedback: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["BAD_REQUEST"], detail="Failed to submit feedback"
        )


@api_router.get("/feedback/summary", response_model=FeedbackSummary)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def get_feedback_summary(
    request: Request,
    tenant: Tenant = Depends(get_current_tenant),
    feedback_service: FeedbackService = Depends(get_feedback_service),
):
    """Get feedback summary for tenant."""
    try:
        summary = await feedback_service.get_feedback_summary(tenant)
        return summary
    except Exception as e:
        logger.error(f"Error getting feedback summary: {e}")
        raise HTTPException(
            status_code=HTTP_STATUS["INTERNAL_ERROR"],
            detail="Failed to get feedback summary",
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
            "database": {"connected": db_healthy, "stats": db_stats},
            "service": "efOfX Estimation Service",
            "version": "1.0.0",
        }
    except Exception as e:
        logger.error(f"Error getting service status: {e}")
        return {"status": "unhealthy", "error": str(e)}


# Admin endpoints (for internal use)
@api_router.get("/admin/tenants")
async def list_tenants(
    tenant: Tenant = Depends(get_current_tenant), limit: int = 10, offset: int = 0
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
            status_code=HTTP_STATUS["FORBIDDEN"], detail="Access denied"
        )


@api_router.get("/admin/reference-classes")
async def list_reference_classes(
    tenant: Tenant = Depends(get_current_tenant), category: Optional[str] = None
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
            status_code=HTTP_STATUS["FORBIDDEN"], detail="Access denied"
        )
