"""
Estimation service for efOfX Estimation Service.

This module contains the core business logic for project estimation
using Reference Class Forecasting (RCF) and LLM integration.
"""

import uuid
import logging
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any
from fastapi import UploadFile

from app.core.config import settings
from app.core.constants import EstimationStatus, API_MESSAGES, ESTIMATION_CONFIG, DB_COLLECTIONS, Region as RegionEnum
from app.models.tenant import Tenant
from app.models.estimation import EstimationResponse, EstimationSession, EstimationOutput
from app.models.chat import ChatSession, ScopingContext
from app.db.mongodb import get_tenant_collection
from app.services.llm_service import LLMService
from app.services.reference_service import ReferenceService
from app.services.prompt_service import PromptService

logger = logging.getLogger(__name__)


class EstimationService:
    """Service for handling estimation logic and sessions."""

    def __init__(self, llm_service: LLMService):
        self.llm_service = llm_service
        self.reference_service = ReferenceService()

    def _collection(self, tenant_id: str):
        """Get tenant-scoped estimates collection."""
        return get_tenant_collection(DB_COLLECTIONS["ESTIMATES"], tenant_id)

    async def get_estimation(self, session_id: str, tenant: Tenant) -> EstimationResponse:
        """Get estimation session status and results."""
        try:
            collection = self._collection(tenant.tenant_id)
            # TenantAwareCollection auto-injects tenant_id — no need to add it manually
            session_data = await collection.find_one({"session_id": session_id})

            if not session_data:
                raise ValueError("Estimation session not found")

            session = EstimationSession(**session_data)

            # Check if session has expired
            if session.expires_at and datetime.utcnow() > session.expires_at:
                session.status = EstimationStatus.EXPIRED
                await collection.update_one(
                    {"session_id": session_id},
                    {"$set": {"status": EstimationStatus.EXPIRED}}
                )
            
            return EstimationResponse(
                session_id=session_id,
                status=session.status,
                message=API_MESSAGES.get(f"ESTIMATION_{session.status.upper()}", "Estimation status retrieved"),
                result=session.result,
                next_action=None if session.status == EstimationStatus.COMPLETED else "wait_for_completion",
                estimated_completion=session.completed_at
            )
            
        except Exception as e:
            logger.error(f"Error getting estimation: {e}")
            raise
    
    async def upload_image(self, session_id: str, file: UploadFile, tenant: Tenant) -> str:
        """Upload image for estimation session."""
        try:
            # Validate file type
            if file.content_type not in settings.ALLOWED_IMAGE_TYPES:
                raise ValueError("Invalid file type")

            # Validate file size
            if file.size > settings.MAX_FILE_SIZE:
                raise ValueError("File too large")

            # Generate image URL (in production, this would upload to cloud storage)
            image_url = f"https://storage.efofx.ai/images/{session_id}/{file.filename}"

            collection = self._collection(tenant.tenant_id)
            # TenantAwareCollection auto-injects tenant_id into the filter
            await collection.update_one(
                {"session_id": session_id},
                {"$push": {"images": image_url}}
            )
            
            return image_url
            
        except Exception as e:
            logger.error(f"Error uploading image: {e}")
            raise
    
    async def _classify_project(self, description: str, region: str) -> str:
        """Classify project using LLM."""
        try:
            # Get available reference classes
            reference_classes = await self.reference_service.get_reference_classes()
            
            # Create classification prompt
            prompt = f"""
            Analyze the following project description and classify it into the most appropriate reference class.
            
            Project Description: {description}
            Region: {region}
            
            Available Reference Classes: {[rc.name for rc in reference_classes]}
            
            Please provide only the reference class name as your response.
            """
            
            # Get LLM response
            response = await self.llm_service.generate_response(prompt)
            
            # Extract reference class from response
            reference_class = response.strip().lower()
            
            # Validate reference class exists
            valid_classes = [rc.name for rc in reference_classes]
            if reference_class not in valid_classes:
                # Default to first available class
                reference_class = valid_classes[0] if valid_classes else "general"
            
            return reference_class
            
        except Exception as e:
            logger.error(f"Error classifying project: {e}")
            return "general"
    
    async def generate_from_chat(
        self,
        session: ChatSession,
        tenant: Tenant,
    ) -> tuple[EstimationSession, EstimationOutput]:
        """Generate a structured estimation from a completed chat scoping session.

        Returns (EstimationSession, EstimationOutput) for use by the streaming endpoint.
        The EstimationSession is saved to MongoDB with prompt_version recorded.
        """
        ctx = session.scoping_context

        # Build description from scoping context
        description = self._build_description_from_context(ctx)
        region = ctx.location or "General"

        # Load estimation prompt (to record prompt_version)
        prompt = PromptService.get("estimation", "latest")
        prompt_version = prompt["version"]

        # Classify project using LLM
        reference_class = await self._classify_project(description, region)

        # Get reference projects
        reference_projects = await self.reference_service.get_reference_projects(
            reference_class, region
        )

        # Generate structured estimation using LLM
        estimation_output = await self.llm_service.generate_estimation(
            description=description,
            reference_class=reference_class,
            region=region,
            reference_data={"reference_projects": reference_projects[:5]} if reference_projects else None,
        )

        # Resolve region to a valid Region enum value if possible
        try:
            resolved_region = RegionEnum(region)
        except ValueError:
            resolved_region = RegionEnum.NORCAL_BAY_AREA  # sensible default

        # Create and save estimation session
        session_id = f"sess_{uuid.uuid4().hex[:12]}"
        est_session = EstimationSession(
            tenant_id=tenant.tenant_id,
            session_id=session_id,
            status=EstimationStatus.COMPLETED,
            description=description,
            region=resolved_region,
            reference_class=reference_class,
            confidence_threshold=0.7,
            prompt_version=prompt_version,
            completed_at=datetime.utcnow(),
            expires_at=datetime.utcnow() + timedelta(minutes=settings.SESSION_TIMEOUT_MINUTES),
        )

        collection = self._collection(tenant.tenant_id)
        await collection.insert_one(est_session.model_dump(by_alias=True))

        return est_session, estimation_output

    def _build_description_from_context(self, ctx: ScopingContext) -> str:
        """Build a natural-language description from scoping context fields."""
        parts = []
        if ctx.project_type:
            parts.append(f"Project type: {ctx.project_type}")
        if ctx.project_size:
            parts.append(f"Size/scope: {ctx.project_size}")
        if ctx.location:
            parts.append(f"Location: {ctx.location}")
        if ctx.timeline:
            parts.append(f"Timeline: {ctx.timeline}")
        if ctx.special_conditions:
            parts.append(f"Special conditions: {ctx.special_conditions}")
        return (". ".join(parts) + ".") if parts else "General project"

