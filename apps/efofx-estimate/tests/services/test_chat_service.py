"""
Unit tests for ChatService — conversation state machine, context extraction, readiness detection.

Mock strategy:
- AsyncMock for LLMService.generate_response
- AsyncMock for MongoDB collection operations (find_one, update_one, insert_one)
- Direct ScopingContext construction for readiness detection unit tests

All tests are isolated from MongoDB and LLM calls.
"""

import pytest
from datetime import datetime, timedelta
from unittest.mock import AsyncMock, MagicMock, patch, call

from app.models.chat import (
    ChatRequest,
    ChatSession,
    ChatMessage,
    ScopingContext,
)
from app.models.tenant import Tenant


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_tenant(tenant_id: str = "tenant-test-001") -> Tenant:
    """Create a minimal Tenant for testing."""
    return Tenant(
        tenant_id=tenant_id,
        company_name="Test Construction Co",
        email="test@example.com",
        hashed_password="$2b$12$hashed",
        hashed_api_key="$2b$12$hashed_api",
        tier="trial",
    )


def make_llm_service(response: str = "What type of project are you planning?") -> MagicMock:
    """Create a mock LLMService that returns a predefined response."""
    mock_llm = MagicMock()
    mock_llm.generate_response = AsyncMock(return_value=response)
    return mock_llm


def make_chat_request(message: str, session_id: str = None) -> ChatRequest:
    """Create a ChatRequest for testing."""
    return ChatRequest(message=message, session_id=session_id)


def make_existing_session(
    session_id: str = "chat_existing001",
    tenant_id: str = "tenant-test-001",
    messages: list = None,
    scoping_context: ScopingContext = None,
    is_ready: bool = False,
    status: str = "active",
) -> dict:
    """Return a session dict as it would come from MongoDB."""
    from datetime import timezone
    session = ChatSession(
        session_id=session_id,
        tenant_id=tenant_id,
        messages=messages or [],
        scoping_context=scoping_context or ScopingContext(),
        is_ready=is_ready,
        status=status,
        prompt_version="1.0.0",
    )
    return session.model_dump(by_alias=True)


# ---------------------------------------------------------------------------
# Fixture: load PromptService with test scoping prompt
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def load_test_prompt():
    """Load the real scoping prompt into PromptService for all tests."""
    import os
    from app.services.prompt_service import PromptService
    PromptService.clear()
    # Use the real prompts directory
    base = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    prompts_dir = os.path.join(base, "config", "prompts")
    PromptService.load_all(prompts_dir)
    yield
    PromptService.clear()


# ---------------------------------------------------------------------------
# ScopingContext unit tests (pure — no mocks needed)
# ---------------------------------------------------------------------------


class TestScopingContext:
    """Pure unit tests for ScopingContext readiness detection."""

    def test_readiness_detection_not_ready_empty(self):
        """Empty ScopingContext is not ready."""
        ctx = ScopingContext()
        assert ctx.is_ready() is False

    def test_readiness_detection_not_ready_partial(self):
        """Only project_type populated — not ready."""
        ctx = ScopingContext(project_type="pool")
        assert ctx.is_ready() is False

    def test_readiness_detection_not_ready_three_fields(self):
        """Three of four required fields — still not ready."""
        ctx = ScopingContext(project_type="pool", project_size="15x30 feet", location="Bay Area")
        assert ctx.is_ready() is False

    def test_readiness_detection_ready(self):
        """All four required fields populated — ready."""
        ctx = ScopingContext(
            project_type="pool",
            project_size="15x30 feet",
            location="Bay Area",
            timeline="spring 2026",
        )
        assert ctx.is_ready() is True

    def test_readiness_detection_ready_with_special_conditions(self):
        """All four required fields plus special_conditions — ready."""
        ctx = ScopingContext(
            project_type="deck",
            project_size="400 sq ft",
            location="SoCal - Coastal",
            timeline="3 months",
            special_conditions="HOA approval needed",
        )
        assert ctx.is_ready() is True

    def test_populated_fields(self):
        """populated_fields returns only non-None fields."""
        ctx = ScopingContext(project_type="pool", location="Bay Area")
        populated = ctx.populated_fields()
        assert populated == {"project_type", "location"}

    def test_missing_fields_priority_order(self):
        """missing_fields returns fields in priority order."""
        ctx = ScopingContext(project_type="pool")
        missing = ctx.missing_fields()
        assert missing == ["project_size", "location", "timeline", "special_conditions"]

    def test_missing_fields_all_populated(self):
        """No missing fields when all five are set."""
        ctx = ScopingContext(
            project_type="pool",
            project_size="15x30",
            location="Bay Area",
            timeline="spring 2026",
            special_conditions="none",
        )
        assert ctx.missing_fields() == []


# ---------------------------------------------------------------------------
# ChatService unit tests
# ---------------------------------------------------------------------------


class TestChatServiceNewSession:
    """Tests for new session creation on first message."""

    async def test_new_session_created_on_first_message(self):
        """Send message with no session_id -> returns new session_id."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Great! What type of project are you planning?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()
        request = make_chat_request("I want to build something in my backyard.")

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=None)
        mock_collection.insert_one = AsyncMock()
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(request, tenant)

        assert response.session_id.startswith("chat_")
        assert len(response.session_id) == len("chat_") + 12

    async def test_new_session_has_two_messages(self):
        """New session after first message has user + assistant messages."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("What type of project are you planning?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()
        request = make_chat_request("I want to do something with my backyard.")

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=None)
        mock_collection.insert_one = AsyncMock()
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(request, tenant)

        # The response content should be the LLM response
        assert response.content == "What type of project are you planning?"

    async def test_new_session_expires_at_set(self):
        """New session has expires_at approximately 24 hours from now."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("What type of project?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()
        request = make_chat_request("Hello")

        captured_session = {}

        async def capture_insert(doc):
            captured_session.update(doc)

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=None)
        mock_collection.insert_one = AsyncMock(side_effect=capture_insert)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            await service.send_message(request, tenant)

        assert "expires_at" in captured_session
        expires_at = captured_session["expires_at"]
        now = datetime.utcnow()
        # Should be approximately 24 hours from now (within a 5-minute buffer)
        expected_min = now + timedelta(hours=23, minutes=55)
        expected_max = now + timedelta(hours=24, minutes=5)
        assert expected_min < expires_at < expected_max


class TestChatServiceExistingSession:
    """Tests for continuing an existing conversation."""

    async def test_existing_session_continued(self):
        """Sending a second message returns same session_id."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("How large is the pool you have in mind?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        existing_session_data = make_existing_session(
            session_id="chat_existing001",
            messages=[
                ChatMessage(role="user", content="I want to build a pool"),
                ChatMessage(role="assistant", content="What size pool?"),
            ],
        )
        request = make_chat_request("About 15x30 feet.", session_id="chat_existing001")

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session_data)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(request, tenant)

        assert response.session_id == "chat_existing001"

    async def test_messages_accumulated_across_turns(self):
        """Conversation history is preserved across multiple turns."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Where will the pool be located?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        existing_session_data = make_existing_session(
            session_id="chat_existing002",
            messages=[
                ChatMessage(role="user", content="I want to build a pool"),
                ChatMessage(role="assistant", content="How large?"),
            ],
        )
        request = make_chat_request("About 15x30 feet.", session_id="chat_existing002")

        updated_doc = {}

        async def capture_update(filter_doc, update_doc, upsert=False):
            updated_doc.update(update_doc.get("$set", {}))

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session_data)
        mock_collection.update_one = AsyncMock(side_effect=capture_update)

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            await service.send_message(request, tenant)

        # Updated doc should have 4 messages (2 existing + 1 new user + 1 new assistant)
        messages = updated_doc.get("messages", [])
        assert len(messages) == 4


class TestScopingContextExtraction:
    """Tests for context extraction from user messages."""

    async def test_scoping_context_extracts_project_type(self):
        """'I want to build a pool' -> scoping_context.project_type is populated."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("What size pool are you thinking?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=None)
        mock_collection.insert_one = AsyncMock()
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("I want to build a pool in my backyard."), tenant
            )

        assert response.scoping_context is not None
        assert response.scoping_context.project_type == "pool"

    async def test_scoping_context_extracts_size_dimensions(self):
        """'about 15x30 feet' -> scoping_context.project_size is populated."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Where will it be located?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        existing_session = make_existing_session(
            scoping_context=ScopingContext(project_type="pool"),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("About 15x30 feet.", session_id="chat_existing001"), tenant
            )

        assert response.scoping_context is not None
        assert response.scoping_context.project_size is not None
        assert "15" in response.scoping_context.project_size

    async def test_scoping_context_extracts_size_sqft(self):
        """'about 500 sq ft' -> scoping_context.project_size is populated."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Where is the project located?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        existing_session = make_existing_session(
            scoping_context=ScopingContext(project_type="deck"),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("It's about 500 sq ft.", session_id="chat_existing001"), tenant
            )

        assert response.scoping_context is not None
        assert response.scoping_context.project_size is not None
        assert "500" in response.scoping_context.project_size

    async def test_scoping_context_extracts_location(self):
        """'in the Bay Area' -> scoping_context.location is populated."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("When would you like to start?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        existing_session = make_existing_session(
            scoping_context=ScopingContext(project_type="pool", project_size="15x30 feet"),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("We're in the Bay Area.", session_id="chat_existing001"), tenant
            )

        assert response.scoping_context is not None
        assert response.scoping_context.location is not None

    async def test_scoping_context_extracts_timeline(self):
        """'spring 2026' -> scoping_context.timeline is populated."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Any special conditions?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        existing_session = make_existing_session(
            scoping_context=ScopingContext(project_type="pool", project_size="15x30", location="Bay Area"),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("We want to start in spring 2026.", session_id="chat_existing001"), tenant
            )

        assert response.scoping_context is not None
        assert response.scoping_context.timeline is not None
        assert "spring" in response.scoping_context.timeline.lower()

    async def test_scoping_context_does_not_overwrite_existing(self):
        """If project_type already set, new message with different type doesn't overwrite."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("How large is the pool?")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        # Session with project_type already set to "pool"
        existing_session = make_existing_session(
            scoping_context=ScopingContext(project_type="pool"),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("I'm also thinking about a deck renovation.", session_id="chat_existing001"),
                tenant,
            )

        # project_type should still be "pool" — not overwritten
        assert response.scoping_context is not None
        assert response.scoping_context.project_type == "pool"


class TestReadinessDetection:
    """Tests for auto-trigger and readiness state transitions."""

    async def test_readiness_transitions_when_all_fields_present(self):
        """When all four required fields are populated, is_ready transitions to True."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Thanks for the details!")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        # Session with 3 of 4 required fields
        existing_session = make_existing_session(
            scoping_context=ScopingContext(
                project_type="pool",
                project_size="15x30 feet",
                location="Bay Area",
                # timeline is missing
            ),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("We want to start in spring 2026.", session_id="chat_existing001"),
                tenant,
            )

        assert response.is_ready is True

    async def test_auto_trigger_message_when_ready(self):
        """When readiness transitions to True, response contains estimate-ready language."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Perfect timing!")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        # Session with 3 of 4 required fields — adding timeline will trigger readiness
        existing_session = make_existing_session(
            scoping_context=ScopingContext(
                project_type="pool",
                project_size="15x30 feet",
                location="Bay Area",
            ),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("We want to start in spring 2026.", session_id="chat_existing001"),
                tenant,
            )

        # Response should contain the auto-trigger phrase
        assert "estimate" in response.content.lower()
        assert response.is_ready is True

    async def test_explicit_estimate_trigger(self):
        """Sending 'generate estimate' -> status='ready'."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Sure, let me generate that.")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        existing_session = make_existing_session(
            scoping_context=ScopingContext(project_type="pool"),
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("generate estimate", session_id="chat_existing001"), tenant
            )

        assert response.status == "ready"
        assert response.is_ready is True
        # LLM should NOT be called for explicit trigger
        mock_llm.generate_response.assert_not_called()

    async def test_user_confirmation_triggers_ready(self):
        """After auto-trigger, user says 'yes' -> status='ready'."""
        from app.services.chat_service import ChatService

        mock_llm = make_llm_service("Should not be called")
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        # Session is already ready (is_ready=True)
        existing_session = make_existing_session(
            scoping_context=ScopingContext(
                project_type="pool",
                project_size="15x30",
                location="Bay Area",
                timeline="spring 2026",
            ),
            is_ready=True,
            status="ready",
        )

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=existing_session)
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(
                make_chat_request("yes", session_id="chat_existing001"), tenant
            )

        assert response.status == "ready"
        assert response.is_ready is True
        # LLM should NOT be called for confirmation
        mock_llm.generate_response.assert_not_called()


class TestIsExplicitEstimateTrigger:
    """Unit tests for _is_explicit_estimate_trigger (pure)."""

    def test_generate_estimate_triggers(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        assert service._is_explicit_estimate_trigger("generate estimate") is True

    def test_give_me_an_estimate_triggers(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        assert service._is_explicit_estimate_trigger("Can you give me an estimate?") is True

    def test_slash_estimate_triggers(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        assert service._is_explicit_estimate_trigger("/estimate") is True

    def test_regular_message_does_not_trigger(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        assert service._is_explicit_estimate_trigger("I want a pool in my backyard.") is False


class TestIsConfirmation:
    """Unit tests for _is_confirmation (pure)."""

    def test_yes_is_confirmation(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        assert service._is_confirmation("yes") is True

    def test_yeah_is_confirmation(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        assert service._is_confirmation("yeah") is True

    def test_go_ahead_is_confirmation(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        assert service._is_confirmation("go ahead") is True

    def test_long_message_is_not_confirmation(self):
        from app.services.chat_service import ChatService
        service = ChatService(llm_service=MagicMock())
        # A longer message indicating additional info is not a simple confirmation
        assert service._is_confirmation("I want to add a spa as well") is False


class TestConversationPreservationOnError:
    """Tests for error handling and conversation preservation."""

    async def test_conversation_preserved_on_llm_error(self):
        """Mock LLM to raise OpenAIError, verify messages still saved to DB."""
        from app.services.chat_service import ChatService
        from openai import OpenAIError

        mock_llm = MagicMock()
        mock_llm.generate_response = AsyncMock(side_effect=OpenAIError("connection error"))
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()
        request = make_chat_request("I want a pool.")

        updated_doc = {}

        async def capture_update(filter_doc, update_doc, upsert=False):
            updated_doc.update(update_doc.get("$set", {}))

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=None)
        mock_collection.insert_one = AsyncMock()
        mock_collection.update_one = AsyncMock(side_effect=capture_update)

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            response = await service.send_message(request, tenant)

        # Response should be an error message, not a raise
        assert "trouble" in response.content.lower() or "unable" in response.content.lower()

        # Conversation must be persisted even on error
        mock_collection.update_one.assert_called_once()
        # The saved session should contain at least 2 messages (user + error response)
        saved_messages = updated_doc.get("messages", [])
        assert len(saved_messages) >= 2
        assert saved_messages[0]["role"] == "user"
        assert saved_messages[1]["role"] == "assistant"

    async def test_response_returned_not_raised_on_error(self):
        """LLM error returns ChatResponse with error message instead of raising."""
        from app.services.chat_service import ChatService
        from openai import OpenAIError

        mock_llm = MagicMock()
        mock_llm.generate_response = AsyncMock(side_effect=OpenAIError("timeout"))
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        mock_collection = AsyncMock()
        mock_collection.find_one = AsyncMock(return_value=None)
        mock_collection.insert_one = AsyncMock()
        mock_collection.update_one = AsyncMock()

        with patch("app.services.chat_service.get_tenant_collection", return_value=mock_collection):
            # Should NOT raise — should return a ChatResponse
            response = await service.send_message(make_chat_request("I want a pool."), tenant)

        assert response is not None
        assert response.session_id is not None
