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
from app.core.constants import EstimationStatus, API_MESSAGES, ESTIMATION_CONFIG, DB_COLLECTIONS
from app.models.tenant import Tenant
from app.models.estimation import EstimationRequest, EstimationResponse, EstimationSession, EstimationResult
from app.db.mongodb import get_tenant_collection
from app.services.llm_service import LLMService
from app.services.reference_service import ReferenceService

logger = logging.getLogger(__name__)


class EstimationService:
    """Service for handling estimation logic and sessions."""

    def __init__(self, llm_service: LLMService):
        self.llm_service = llm_service
        self.reference_service = ReferenceService()

    def _collection(self, tenant_id: str):
        """Get tenant-scoped estimates collection."""
        return get_tenant_collection(DB_COLLECTIONS["ESTIMATES"], tenant_id)

    async def start_estimation(self, request: EstimationRequest, tenant: Tenant) -> EstimationResponse:
        """Start a new estimation session."""
        try:
            # Generate session ID
            session_id = f"sess_{uuid.uuid4().hex[:12]}"

            # Create estimation session
            session = EstimationSession(
                tenant_id=tenant.id,
                session_id=session_id,
                status=EstimationStatus.INITIATED,
                description=request.description,
                region=request.region,
                reference_class=request.reference_class,
                confidence_threshold=request.confidence_threshold,
                expires_at=datetime.utcnow() + timedelta(minutes=settings.SESSION_TIMEOUT_MINUTES)
            )

            collection = self._collection(tenant.id)
            # Save to database
            await collection.insert_one(session.dict(by_alias=True))
            
            # Start estimation process (pass collection to avoid re-creating it)
            result = await self._process_estimation(session, tenant)
            
            return EstimationResponse(
                session_id=session_id,
                status=result.status,
                message=API_MESSAGES["ESTIMATION_STARTED"],
                result=result.result,
                next_action="wait_for_completion",
                estimated_completion=datetime.utcnow() + timedelta(minutes=5)
            )
            
        except Exception as e:
            logger.error(f"Error starting estimation: {e}")
            raise
    
    async def get_estimation(self, session_id: str, tenant: Tenant) -> EstimationResponse:
        """Get estimation session status and results."""
        try:
            collection = self._collection(tenant.id)
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

            collection = self._collection(tenant.id)
            # TenantAwareCollection auto-injects tenant_id into the filter
            await collection.update_one(
                {"session_id": session_id},
                {"$push": {"images": image_url}}
            )
            
            return image_url
            
        except Exception as e:
            logger.error(f"Error uploading image: {e}")
            raise
    
    async def _process_estimation(self, session: EstimationSession, tenant: Tenant) -> EstimationSession:
        """Process estimation using RCF and LLM."""
        collection = self._collection(tenant.id)
        try:
            # Update status to in progress
            session.status = EstimationStatus.IN_PROGRESS
            await collection.update_one(
                {"session_id": session.session_id},
                {"$set": {"status": EstimationStatus.IN_PROGRESS}}
            )

            # Classify project using LLM
            reference_class = await self._classify_project(session.description, session.region)

            # Get reference projects
            reference_projects = await self.reference_service.get_reference_projects(
                reference_class, session.region
            )

            # Generate estimation using LLM
            estimation_result = await self._generate_estimation(
                session.description,
                session.region,
                reference_class,
                reference_projects
            )

            # Update session with results
            session.reference_class = reference_class
            session.result = estimation_result
            session.status = EstimationStatus.COMPLETED
            session.completed_at = datetime.utcnow()

            # Save updated session
            await collection.update_one(
                {"session_id": session.session_id},
                {"$set": {
                    "reference_class": reference_class,
                    "result": estimation_result.dict(),
                    "status": EstimationStatus.COMPLETED,
                    "completed_at": session.completed_at,
                    "updated_at": datetime.utcnow()
                }}
            )

            return session

        except Exception as e:
            logger.error(f"Error processing estimation: {e}")
            # Update status to failed
            await collection.update_one(
                {"session_id": session.session_id},
                {"$set": {"status": "failed"}}
            )
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
    
    async def _generate_estimation(
        self, 
        description: str, 
        region: str, 
        reference_class: str, 
        reference_projects: List[Dict[str, Any]]
    ) -> EstimationResult:
        """Generate estimation using LLM and reference data."""
        try:
            # Create estimation prompt
            prompt = f"""
            Based on the following project details and reference data, generate a comprehensive estimate.
            
            Project Details:
            - Description: {description}
            - Region: {region}
            - Reference Class: {reference_class}
            
            Reference Projects: {reference_projects[:5]}  # Use top 5 most relevant
            
            Please provide a structured estimate with:
            1. Total estimated cost
            2. Estimated timeline (weeks)
            3. Recommended team size
            4. Cost breakdown by category (materials, labor, equipment, permits, design, contingency, profit_margin)
            5. Confidence score (0-1)
            6. Key assumptions
            7. Identified risks
            """
            
            # Get LLM response
            response = await self.llm_service.generate_response(prompt)
            
            # Parse response and create estimation result
            # This is a simplified version - in production, you'd have more sophisticated parsing
            estimation_result = self._parse_estimation_response(response, reference_projects)
            
            return estimation_result
            
        except Exception as e:
            logger.error(f"Error generating estimation: {e}")
            # Return default estimation
            return self._create_default_estimation(description, region, reference_class)
    
    def _parse_estimation_response(self, response: str, reference_projects: List[Dict[str, Any]]) -> EstimationResult:
        """Parse LLM response into structured estimation result."""
        # This is a simplified parser - in production, you'd use more sophisticated parsing
        # or structured output from the LLM
        
        # Calculate average from reference projects
        avg_cost = sum(p.get("total_cost", 50000) for p in reference_projects) / len(reference_projects) if reference_projects else 50000
        avg_timeline = sum(p.get("timeline_weeks", 8) for p in reference_projects) / len(reference_projects) if reference_projects else 8
        avg_team_size = sum(p.get("team_size", 4) for p in reference_projects) / len(reference_projects) if reference_projects else 4
        
        # Create cost breakdown (simplified)
        cost_breakdown = {
            "materials": avg_cost * 0.4,
            "labor": avg_cost * 0.25,
            "equipment": avg_cost * 0.08,
            "permits": avg_cost * 0.03,
            "design": avg_cost * 0.05,
            "contingency": avg_cost * 0.08,
            "profit_margin": avg_cost * 0.11
        }
        
        return EstimationResult(
            total_cost=avg_cost,
            timeline_weeks=int(avg_timeline),
            team_size=int(avg_team_size),
            cost_breakdown=cost_breakdown,
            reference_class="general",
            confidence_score=0.8,
            assumptions=["Standard project scope", "No major site preparation required"],
            risks=["Weather delays", "Material cost fluctuations"],
            reference_projects_used=[p.get("project_id", "") for p in reference_projects[:3]]
        )
    
    def _create_default_estimation(self, description: str, region: str, reference_class: str) -> EstimationResult:
        """Create default estimation when LLM processing fails."""
        return EstimationResult(
            total_cost=50000.0,
            timeline_weeks=8,
            team_size=4,
            cost_breakdown={
                "materials": 20000.0,
                "labor": 12500.0,
                "equipment": 4000.0,
                "permits": 1500.0,
                "design": 2500.0,
                "contingency": 4000.0,
                "profit_margin": 5500.0
            },
            reference_class=reference_class,
            confidence_score=0.5,
            assumptions=["Default estimation due to processing error"],
            risks=["Estimation accuracy may be lower than usual"],
            reference_projects_used=[]
        ) 