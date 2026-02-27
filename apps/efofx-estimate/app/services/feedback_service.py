"""
Feedback service for efOfX Estimation Service.

This module provides feedback functionality for collecting user feedback
and improving estimation accuracy over time.
"""

import logging
from datetime import datetime, timedelta
from typing import List, Dict, Any

from app.models.tenant import Tenant
from app.models.feedback import FeedbackCreate, FeedbackSummary, Feedback
from app.db.mongodb import get_tenant_collection
from app.core.constants import DB_COLLECTIONS

logger = logging.getLogger(__name__)


class FeedbackService:
    """Service for handling feedback functionality."""

    def _collection(self, tenant_id: str):
        """Get tenant-scoped feedback collection."""
        return get_tenant_collection(DB_COLLECTIONS["FEEDBACK"], tenant_id)

    async def submit_feedback(self, feedback: FeedbackCreate, tenant: Tenant) -> str:
        """Submit new feedback."""
        try:
            # Create feedback document
            feedback_doc = Feedback(
                tenant_id=tenant.tenant_id,
                estimation_session_id=feedback.estimation_session_id,
                feedback_type=feedback.feedback_type,
                rating=feedback.rating,
                comment=feedback.comment,
                actual_cost=feedback.actual_cost,
                actual_timeline=feedback.actual_timeline,
                actual_team_size=feedback.actual_team_size,
                cost_accuracy=feedback.cost_accuracy,
                timeline_accuracy=feedback.timeline_accuracy,
                reference_class_accuracy=feedback.reference_class_accuracy,
                metadata=feedback.metadata or {}
            )

            collection = self._collection(tenant.tenant_id)
            # Save to database — TenantAwareCollection stamps tenant_id automatically
            result = await collection.insert_one(feedback_doc.dict(by_alias=True))
            
            logger.info(f"Feedback submitted successfully: {result.inserted_id}")
            return str(result.inserted_id)
            
        except Exception as e:
            logger.error(f"Error submitting feedback: {e}")
            raise
    
    async def get_feedback_summary(self, tenant: Tenant) -> FeedbackSummary:
        """Get feedback summary for tenant."""
        try:
            collection = self._collection(tenant.tenant_id)
            # TenantAwareCollection auto-scopes to this tenant — no filter needed
            feedback_cursor = collection.find({})
            feedback_list = await feedback_cursor.to_list(length=None)
            
            if not feedback_list:
                return FeedbackSummary(
                    total_feedback=0,
                    average_rating=0.0,
                    cost_accuracy_avg=None,
                    timeline_accuracy_avg=None,
                    reference_class_accuracy_avg=None,
                    feedback_by_type={},
                    recent_feedback=[]
                )
            
            # Calculate statistics
            total_feedback = len(feedback_list)
            average_rating = sum(f.get("rating", 0) for f in feedback_list) / total_feedback
            
            # Calculate accuracy averages
            cost_accuracies = [f.get("cost_accuracy") for f in feedback_list if f.get("cost_accuracy")]
            timeline_accuracies = [f.get("timeline_accuracy") for f in feedback_list if f.get("timeline_accuracy")]
            ref_class_accuracies = [f.get("reference_class_accuracy") for f in feedback_list if f.get("reference_class_accuracy")]
            
            cost_accuracy_avg = sum(cost_accuracies) / len(cost_accuracies) if cost_accuracies else None
            timeline_accuracy_avg = sum(timeline_accuracies) / len(timeline_accuracies) if timeline_accuracies else None
            reference_class_accuracy_avg = sum(ref_class_accuracies) / len(ref_class_accuracies) if ref_class_accuracies else None
            
            # Group by feedback type
            feedback_by_type = {}
            for f in feedback_list:
                feedback_type = f.get("feedback_type", "unknown")
                feedback_by_type[feedback_type] = feedback_by_type.get(feedback_type, 0) + 1
            
            # Get recent feedback (last 10)
            recent_feedback = [
                Feedback(**f) for f in sorted(
                    feedback_list, 
                    key=lambda x: x.get("created_at", datetime.min), 
                    reverse=True
                )[:10]
            ]
            
            return FeedbackSummary(
                total_feedback=total_feedback,
                average_rating=average_rating,
                cost_accuracy_avg=cost_accuracy_avg,
                timeline_accuracy_avg=timeline_accuracy_avg,
                reference_class_accuracy_avg=reference_class_accuracy_avg,
                feedback_by_type=feedback_by_type,
                recent_feedback=recent_feedback
            )
            
        except Exception as e:
            logger.error(f"Error getting feedback summary: {e}")
            raise
    
    async def get_feedback_by_type(self, tenant: Tenant, feedback_type: str) -> List[Feedback]:
        """Get feedback by type for tenant."""
        try:
            collection = self._collection(tenant.tenant_id)
            cursor = collection.find({"feedback_type": feedback_type})
            
            feedback_list = await cursor.to_list(length=None)
            return [Feedback(**f) for f in feedback_list]
            
        except Exception as e:
            logger.error(f"Error getting feedback by type: {e}")
            raise
    
    async def get_feedback_by_session(self, session_id: str, tenant: Tenant) -> List[Feedback]:
        """Get feedback for specific estimation session."""
        try:
            collection = self._collection(tenant.tenant_id)
            cursor = collection.find({"estimation_session_id": session_id})
            
            feedback_list = await cursor.to_list(length=None)
            return [Feedback(**f) for f in feedback_list]
            
        except Exception as e:
            logger.error(f"Error getting feedback by session: {e}")
            raise
    
    async def get_feedback_analytics(self, tenant: Tenant, days: int = 30) -> Dict[str, Any]:
        """Get feedback analytics for tenant."""
        try:
            # Get feedback from last N days
            start_date = datetime.utcnow() - timedelta(days=days)

            collection = self._collection(tenant.tenant_id)
            cursor = collection.find({"created_at": {"$gte": start_date}})
            
            feedback_list = await cursor.to_list(length=None)
            
            if not feedback_list:
                return {
                    "period_days": days,
                    "total_feedback": 0,
                    "average_rating": 0.0,
                    "accuracy_trends": {},
                    "feedback_distribution": {}
                }
            
            # Calculate analytics
            total_feedback = len(feedback_list)
            average_rating = sum(f.get("rating", 0) for f in feedback_list) / total_feedback
            
            # Calculate accuracy trends
            accuracy_trends = {
                "cost_accuracy": self._calculate_accuracy_trend(feedback_list, "cost_accuracy"),
                "timeline_accuracy": self._calculate_accuracy_trend(feedback_list, "timeline_accuracy"),
                "reference_class_accuracy": self._calculate_accuracy_trend(feedback_list, "reference_class_accuracy")
            }
            
            # Calculate feedback distribution
            feedback_distribution = {}
            for f in feedback_list:
                feedback_type = f.get("feedback_type", "unknown")
                feedback_distribution[feedback_type] = feedback_distribution.get(feedback_type, 0) + 1
            
            return {
                "period_days": days,
                "total_feedback": total_feedback,
                "average_rating": average_rating,
                "accuracy_trends": accuracy_trends,
                "feedback_distribution": feedback_distribution
            }
            
        except Exception as e:
            logger.error(f"Error getting feedback analytics: {e}")
            raise
    
    def _calculate_accuracy_trend(self, feedback_list: List[Dict], accuracy_field: str) -> Dict[str, float]:
        """Calculate accuracy trend over time."""
        # Group by week and calculate average accuracy
        weekly_accuracies = {}
        
        for f in feedback_list:
            if f.get(accuracy_field):
                # Get week of feedback
                created_at = f.get("created_at", datetime.utcnow())
                week_start = created_at - timedelta(days=created_at.weekday())
                week_key = week_start.strftime("%Y-%W")
                
                if week_key not in weekly_accuracies:
                    weekly_accuracies[week_key] = []
                
                weekly_accuracies[week_key].append(f[accuracy_field])
        
        # Calculate weekly averages
        weekly_averages = {}
        for week, accuracies in weekly_accuracies.items():
            weekly_averages[week] = sum(accuracies) / len(accuracies)
        
        return weekly_averages 