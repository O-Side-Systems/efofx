"""
Chat service for efOfX Estimation Service.

This module provides chat functionality for conversational estimation
and session management. The ChatService implements a conversation state
machine that guides users through structured project scoping with
LLM-powered follow-up questions.
"""

import re
import uuid
import logging
from datetime import datetime
from typing import List, Dict, Any, Optional

from openai import OpenAIError

from app.models.tenant import Tenant
from app.models.chat import (
    ChatRequest,
    ChatResponse,
    ChatMessage,
    ChatSession,
    ScopingContext,
)
from app.db.mongodb import get_tenant_collection
from app.core.constants import DB_COLLECTIONS
from app.services.llm_service import LLMService
from app.services.prompt_service import PromptService

logger = logging.getLogger(__name__)


class ChatService:
    """Service for handling conversational project scoping.

    Implements a state machine that:
    1. Maintains full multi-turn conversation history in MongoDB
    2. Uses PromptService to load the scoping prompt
    3. Uses LLMService to generate context-aware follow-up questions
    4. Extracts scoping context via keyword/pattern matching
    5. Detects readiness via rule-based field check
    6. Auto-triggers estimate confirmation when all required fields are populated
    7. Preserves conversation across LLM errors
    """

    ESTIMATE_TRIGGER_PHRASES = {
        "generate estimate",
        "give me an estimate",
        "ready for estimate",
        "create estimate",
        "/estimate",
    }

    CONFIRMATION_WORDS = {
        "yes", "yeah", "sure", "go ahead", "ready", "go", "yep", "do it",
        "ok", "okay", "let's go", "absolutely", "definitely", "please",
    }

    def __init__(self, llm_service: LLMService):
        self.llm_service = llm_service

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    async def send_message(self, request: ChatRequest, tenant: Tenant) -> ChatResponse:
        """Send a chat message and get an LLM-powered response.

        Conversation flow:
        1. Get or create session
        2. Append user message
        3. Check for explicit estimate trigger
        4. If triggered/confirmed: transition to ready state
        5. Otherwise: generate LLM follow-up, extract context, check readiness
        6. Persist session
        7. Return response
        """
        # Get or create session
        session = await self._get_or_create_session(request.session_id, tenant)

        # Append user message to history
        session.messages.append(ChatMessage(role="user", content=request.message))

        # Handle explicit estimate trigger
        if self._is_explicit_estimate_trigger(request.message):
            session.status = "ready"
            session.is_ready = True
            response_content = (
                "Got it! I'll start generating your estimate now. "
                "This will just take a moment."
            )
            session.messages.append(ChatMessage(role="assistant", content=response_content))
            await self._persist_session(session, tenant)
            return ChatResponse(
                session_id=session.session_id,
                content=response_content,
                is_ready=True,
                scoping_context=session.scoping_context,
                status="ready",
            )

        # Handle user confirmation when system is already ready
        if session.is_ready and self._is_confirmation(request.message):
            session.status = "ready"
            response_content = (
                "Great! I'll start generating your estimate now. "
                "This will just take a moment."
            )
            session.messages.append(ChatMessage(role="assistant", content=response_content))
            await self._persist_session(session, tenant)
            return ChatResponse(
                session_id=session.session_id,
                content=response_content,
                is_ready=True,
                scoping_context=session.scoping_context,
                status="ready",
            )

        # Generate LLM follow-up question
        try:
            llm_response = await self._generate_follow_up(session)
        except OpenAIError as e:
            error_type_str = str(type(e).__name__)
            if "Auth" in error_type_str or "quota" in str(e).lower():
                logger.error(f"OpenAI auth/quota error: {e}")
                response_content = (
                    "We're unable to process your request due to an API key issue. "
                    "Please check your OpenAI API key in Settings."
                )
            else:
                logger.warning(f"Transient OpenAI error: {e}")
                response_content = (
                    "We're having trouble generating a response. Please try again in a moment."
                )
            # Conversation is ALWAYS preserved — remove the last user message wasn't added yet
            session.messages.append(ChatMessage(role="assistant", content=response_content))
            await self._persist_session(session, tenant)
            return ChatResponse(
                session_id=session.session_id,
                content=response_content,
                is_ready=session.is_ready,
                scoping_context=session.scoping_context,
                status=session.status,
            )
        except Exception as e:
            logger.error(f"Unexpected error generating follow-up: {e}")
            response_content = (
                "We're having trouble generating a response. Please try again in a moment."
            )
            session.messages.append(ChatMessage(role="assistant", content=response_content))
            await self._persist_session(session, tenant)
            return ChatResponse(
                session_id=session.session_id,
                content=response_content,
                is_ready=session.is_ready,
                scoping_context=session.scoping_context,
                status=session.status,
            )

        # Extract context from user message
        self._update_scoping_context(session, request.message, llm_response)

        # Check if session just became ready
        was_ready = session.is_ready
        if session.scoping_context.is_ready():
            session.is_ready = True
            if not was_ready:
                # Append auto-trigger message
                llm_response = (
                    llm_response.rstrip()
                    + "\n\nI have enough details to generate an estimate for your project. Shall I go ahead?"
                )
                session.status = "ready"

        # Append assistant message
        session.messages.append(ChatMessage(role="assistant", content=llm_response))

        # Persist session
        await self._persist_session(session, tenant)

        return ChatResponse(
            session_id=session.session_id,
            content=llm_response,
            is_ready=session.is_ready,
            scoping_context=session.scoping_context,
            status=session.status,
        )

    async def get_chat_history(self, session_id: str, tenant: Tenant) -> List[Dict[str, Any]]:
        """Get full chat history for a session."""
        collection = get_tenant_collection(DB_COLLECTIONS["CHAT_SESSIONS"], tenant.tenant_id)
        session_data = await collection.find_one({"session_id": session_id})

        if not session_data:
            raise ValueError("Chat session not found")

        session = ChatSession(**session_data)
        return [msg.model_dump() for msg in session.messages]

    # ------------------------------------------------------------------
    # Private helpers
    # ------------------------------------------------------------------

    async def _get_or_create_session(
        self, session_id: Optional[str], tenant: Tenant
    ) -> ChatSession:
        """Fetch existing session from DB or create a new one."""
        collection = get_tenant_collection(DB_COLLECTIONS["CHAT_SESSIONS"], tenant.tenant_id)

        if session_id:
            session_data = await collection.find_one({"session_id": session_id})
            if session_data:
                return ChatSession(**session_data)

        # Create new session
        new_session_id = f"chat_{uuid.uuid4().hex[:12]}"
        prompt_version = PromptService.get_version_string("scoping", "latest")

        session = ChatSession(
            session_id=new_session_id,
            tenant_id=tenant.tenant_id,
            prompt_version=prompt_version,
        )

        await collection.insert_one(session.model_dump(by_alias=True))
        return session

    async def _generate_follow_up(self, session: ChatSession) -> str:
        """Generate an LLM follow-up question using the scoping prompt."""
        prompt = PromptService.get("scoping", session.prompt_version or "latest")

        # Build conversation history string (last 10 messages for context window)
        recent_messages = session.messages[-10:]
        conversation_history = "\n".join(
            f"{msg.role.upper()}: {msg.content}" for msg in recent_messages
        )

        # Build scoping context string
        ctx = session.scoping_context
        populated = ctx.populated_fields()
        if populated:
            context_parts = []
            for field in ["project_type", "project_size", "location", "timeline", "special_conditions"]:
                value = getattr(ctx, field)
                if value is not None:
                    context_parts.append(f"- {field}: {value}")
            scoping_context_str = "\n".join(context_parts)
        else:
            scoping_context_str = "Nothing gathered yet."

        # Get the last user message
        user_messages = [m for m in session.messages if m.role == "user"]
        last_user_message = user_messages[-1].content if user_messages else ""

        # Format the user prompt template
        formatted_user_prompt = prompt["user_prompt_template"].format(
            conversation_history=conversation_history,
            user_message=last_user_message,
            scoping_context=scoping_context_str,
        )

        response = await self.llm_service.generate_response(
            formatted_user_prompt,
            prompt["system_prompt"],
        )
        return response

    def _update_scoping_context(
        self, session: ChatSession, user_message: str, llm_response: str
    ) -> None:
        """Extract scoping context from user message using keyword/pattern matching.

        Only updates fields that are currently None — does not overwrite existing context.
        """
        msg_lower = user_message.lower()
        ctx = session.scoping_context

        # Project type — check for keywords
        if ctx.project_type is None:
            project_type_keywords = {
                "pool": "pool",
                "swimming pool": "pool",
                "spa": "pool",
                "deck": "deck",
                "patio": "patio",
                "renovation": "renovation",
                "remodel": "renovation",
                "addition": "addition",
                "kitchen": "kitchen renovation",
                "bathroom": "bathroom renovation",
                "bath": "bathroom renovation",
                "roof": "roofing",
                "fence": "fencing",
                "landscaping": "landscaping",
                "landscape": "landscaping",
                "driveway": "driveway",
                "garage": "garage",
                "shed": "shed",
                "pergola": "pergola",
                "outdoor kitchen": "outdoor kitchen",
            }
            for keyword, project_type in project_type_keywords.items():
                if keyword in msg_lower:
                    ctx.project_type = project_type
                    break

        # Project size — regex for dimensions
        if ctx.project_size is None:
            # Match patterns like "15x30", "15 x 30", "500 sq ft", "500 sqft", "500 square feet"
            size_patterns = [
                r"(\d+)\s*x\s*(\d+)\s*(?:feet|ft|foot)?",
                r"(\d+)\s*(?:sq\.?\s*ft|sqft|square\s*feet|square\s*foot)",
            ]
            for pattern in size_patterns:
                match = re.search(pattern, msg_lower)
                if match:
                    if len(match.groups()) == 2:
                        ctx.project_size = f"{match.group(1)}x{match.group(2)} feet"
                    else:
                        ctx.project_size = f"{match.group(1)} sq ft"
                    break

        # Location — check for known regions or general location patterns
        if ctx.location is None:
            location_keywords = {
                "bay area": "NorCal - Bay Area",
                "san francisco": "NorCal - Bay Area",
                "sf": "NorCal - Bay Area",
                "silicon valley": "NorCal - Bay Area",
                "san jose": "NorCal - Bay Area",
                "los angeles": "SoCal - Coastal",
                "la": "SoCal - Coastal",
                "san diego": "SoCal - Coastal",
                "santa barbara": "SoCal - Coastal",
                "orange county": "SoCal - Coastal",
                "socal": "SoCal - Coastal",
                "southern california": "SoCal - Coastal",
                "inland empire": "SoCal - Inland",
                "riverside": "SoCal - Inland",
                "san bernardino": "SoCal - Inland",
                "norcal": "NorCal - Central",
                "northern california": "NorCal - Central",
                "sacramento": "NorCal - Central",
                "fresno": "NorCal - Central",
                "phoenix": "Arizona - Phoenix",
                "scottsdale": "Arizona - Phoenix",
                "tempe": "Arizona - Phoenix",
                "tucson": "Arizona - Tucson",
                "las vegas": "Nevada - Las Vegas",
                "henderson": "Nevada - Las Vegas",
                "reno": "Nevada - Reno",
                "arizona": "Arizona - Phoenix",
                "nevada": "Nevada - Las Vegas",
                "california": "NorCal - Bay Area",
            }
            for keyword, location in location_keywords.items():
                if keyword in msg_lower:
                    ctx.location = location
                    break

            # Fall back to general location mention if specific region not found
            if ctx.location is None:
                location_match = re.search(
                    r"(?:in|near|around|located in|from)\s+([a-z\s,]+?)(?:\s*[.,!?]|$)",
                    msg_lower,
                )
                if location_match:
                    raw_location = location_match.group(1).strip()
                    if len(raw_location) > 2 and raw_location not in {"the", "a", "an"}:
                        ctx.location = raw_location.title()

        # Timeline — check for time references
        if ctx.timeline is None:
            timeline_patterns = [
                r"(?:spring|summer|fall|autumn|winter)\s*(?:\d{4})?",
                r"(?:january|february|march|april|may|june|july|august|september|october|november|december)(?:\s+\d{4})?",
                r"\d+\s*(?:weeks?|months?|years?)",
                r"asap|as soon as possible",
                r"next\s+(?:month|year|spring|summer|fall|winter)",
                r"by\s+(?:the\s+)?(?:end\s+of\s+)?(?:spring|summer|fall|winter|\d{4}|next)",
            ]
            for pattern in timeline_patterns:
                match = re.search(pattern, msg_lower)
                if match:
                    ctx.timeline = match.group(0).strip()
                    break

        # Special conditions — check for common keywords
        if ctx.special_conditions is None:
            special_keywords = [
                "hoa", "homeowners association",
                "slope", "sloped", "hillside",
                "access", "limited access",
                "soil", "clay soil", "sandy",
                "permit", "permits",
                "existing", "removal", "tear out",
                "drainage", "drain",
                "underground", "buried",
                "easement",
                "setback",
            ]
            found_conditions = []
            for keyword in special_keywords:
                if keyword in msg_lower:
                    found_conditions.append(keyword)
            if found_conditions:
                ctx.special_conditions = ", ".join(found_conditions)

    def _is_explicit_estimate_trigger(self, message: str) -> bool:
        """Check if message contains an explicit estimate trigger phrase."""
        message_lower = message.lower().strip()
        return any(trigger in message_lower for trigger in self.ESTIMATE_TRIGGER_PHRASES)

    def _is_confirmation(self, message: str) -> bool:
        """Check if message is a short affirmative response."""
        message_lower = message.lower().strip()
        # Check exact matches and partial matches for multi-word confirmations
        for word in self.CONFIRMATION_WORDS:
            if message_lower == word or message_lower.startswith(word + " ") or message_lower.endswith(" " + word):
                return True
        return False

    async def _persist_session(self, session: ChatSession, tenant: Tenant) -> None:
        """Upsert the session to MongoDB."""
        session.updated_at = datetime.utcnow()
        collection = get_tenant_collection(DB_COLLECTIONS["CHAT_SESSIONS"], tenant.tenant_id)

        # Serialize messages as dicts for MongoDB storage
        session_data = session.model_dump(by_alias=True)

        await collection.update_one(
            {"session_id": session.session_id},
            {"$set": session_data},
            upsert=True,
        )
