"""
Chat service for efOfX Estimation Service.

This module provides chat functionality for conversational estimation
and session management.
"""

import uuid
import logging
from datetime import datetime
from typing import List, Dict, Any, Optional

from app.models.tenant import Tenant
from app.models.chat import ChatRequest, ChatResponse, ChatMessage, ChatSession
from app.db.mongodb import get_tenant_collection
from app.core.constants import DB_COLLECTIONS
from app.services.llm_service import LLMService

logger = logging.getLogger(__name__)


class ChatService:
    """Service for handling chat functionality."""

    def __init__(self, llm_service: LLMService):
        self.llm_service = llm_service

    def _collection(self, tenant_id: str):
        """Get tenant-scoped chat sessions collection."""
        return get_tenant_collection(DB_COLLECTIONS["CHAT_SESSIONS"], tenant_id)

    async def send_message(self, request: ChatRequest, tenant: Tenant) -> ChatResponse:
        """Send a chat message and get response."""
        try:
            # Get or create chat session
            session = await self._get_or_create_session(request.session_id, tenant)
            
            # Create user message
            user_message = ChatMessage(
                session_id=session.session_id,
                message_id=f"msg_{uuid.uuid4().hex[:8]}",
                role="user",
                content=request.message
            )
            
            # Generate assistant response
            assistant_response = await self._generate_chat_response(request.message, session)
            
            # Create assistant message
            assistant_message = ChatMessage(
                session_id=session.session_id,
                message_id=f"msg_{uuid.uuid4().hex[:8]}",
                role="assistant",
                content=assistant_response["content"]
            )
            
            # Update session
            session.messages.extend([user_message.message_id, assistant_message.message_id])
            session.context.update(assistant_response.get("context_updates", {}))
            session.updated_at = datetime.utcnow()
            
            sessions_collection = self._collection(tenant.id)
            # Save to database (in production, you'd save messages separately)
            await sessions_collection.update_one(
                {"session_id": session.session_id},
                {"$set": {
                    "messages": session.messages,
                    "context": session.context,
                    "updated_at": session.updated_at
                }}
            )
            
            return ChatResponse(
                session_id=session.session_id,
                message_id=assistant_message.message_id,
                content=assistant_message.content,
                timestamp=assistant_message.timestamp,
                next_action=assistant_response.get("next_action"),
                context_updates=assistant_response.get("context_updates"),
                is_complete=assistant_response.get("is_complete", False)
            )
            
        except Exception as e:
            logger.error(f"Error sending chat message: {e}")
            raise
    
    async def get_chat_history(self, session_id: str, tenant: Tenant) -> List[Dict[str, Any]]:
        """Get chat history for a session."""
        try:
            sessions_collection = self._collection(tenant.id)
            # TenantAwareCollection auto-injects tenant_id — only session_id filter needed
            session_data = await sessions_collection.find_one({"session_id": session_id})
            
            if not session_data:
                raise ValueError("Chat session not found")
            
            # In production, you'd fetch actual messages from a messages collection
            # For now, return a simplified history
            return [
                {
                    "message_id": f"msg_{i}",
                    "role": "user" if i % 2 == 0 else "assistant",
                    "content": f"Message {i+1}",
                    "timestamp": datetime.utcnow().isoformat()
                }
                for i in range(len(session_data.get("messages", [])))
            ]
            
        except Exception as e:
            logger.error(f"Error getting chat history: {e}")
            raise
    
    async def _get_or_create_session(self, session_id: Optional[str], tenant: Tenant) -> ChatSession:
        """Get existing session or create new one."""
        sessions_collection = self._collection(tenant.id)
        if session_id:
            # TenantAwareCollection auto-injects tenant_id into filter
            session_data = await sessions_collection.find_one({"session_id": session_id})

            if session_data:
                return ChatSession(**session_data)

        # Create new session
        new_session_id = f"chat_sess_{uuid.uuid4().hex[:12]}"
        session = ChatSession(
            session_id=new_session_id,
            tenant_id=tenant.id,
            estimation_session_id="",  # Will be set when estimation starts
            context={"created_at": datetime.utcnow().isoformat()}
        )

        await sessions_collection.insert_one(session.dict(by_alias=True))
        return session
    
    async def _generate_chat_response(self, message: str, session: ChatSession) -> Dict[str, Any]:
        """Generate chat response using LLM."""
        try:
            # Create context-aware prompt
            prompt = f"""
            You are an expert construction estimator helping a client with project estimation.
            
            Current session context: {session.context}
            User message: {message}
            
            Provide a helpful, professional response that:
            1. Acknowledges the user's input
            2. Asks clarifying questions if needed
            3. Provides relevant information about estimation process
            4. Guides them toward completing their project description
            """
            
            system_message = """
            You are a professional construction estimator assistant. Be helpful, knowledgeable,
            and guide users through the estimation process. Ask clarifying questions when needed
            to gather complete project information.
            """
            
            response = await self.llm_service.generate_response(prompt, system_message)
            
            # Determine next action based on message content
            next_action = self._determine_next_action(message, session.context)
            
            # Update context based on message
            context_updates = self._extract_context_updates(message)
            
            return {
                "content": response,
                "next_action": next_action,
                "context_updates": context_updates,
                "is_complete": "complete" in message.lower() or "done" in message.lower()
            }
            
        except Exception as e:
            logger.error(f"Error generating chat response: {e}")
            return {
                "content": "I apologize, but I'm having trouble processing your request. Please try again or contact support.",
                "next_action": "retry",
                "context_updates": {},
                "is_complete": False
            }
    
    def _determine_next_action(self, message: str, context: Dict[str, Any]) -> str:
        """Determine next action based on message content."""
        message_lower = message.lower()
        
        if any(word in message_lower for word in ["pool", "swimming", "backyard"]):
            return "gather_pool_details"
        elif any(word in message_lower for word in ["deck", "patio", "outdoor"]):
            return "gather_outdoor_details"
        elif any(word in message_lower for word in ["renovation", "remodel"]):
            return "gather_renovation_details"
        elif any(word in message_lower for word in ["complete", "done", "finish"]):
            return "complete_estimation"
        else:
            return "gather_general_details"
    
    def _extract_context_updates(self, message: str) -> Dict[str, Any]:
        """Extract context updates from user message."""
        updates = {}
        message_lower = message.lower()
        
        # Extract project type
        if "pool" in message_lower:
            updates["project_type"] = "residential_pool"
        elif "deck" in message_lower or "patio" in message_lower:
            updates["project_type"] = "outdoor_structure"
        elif "renovation" in message_lower or "remodel" in message_lower:
            updates["project_type"] = "renovation"
        
        # Extract size information
        import re
        size_match = re.search(r'(\d+)\s*x\s*(\d+)', message_lower)
        if size_match:
            updates["project_size"] = f"{size_match.group(1)}x{size_match.group(2)}"
        
        # Extract region information
        regions = ["socal", "norcal", "arizona", "nevada"]
        for region in regions:
            if region in message_lower:
                updates["region"] = region
                break
        
        return updates 