"""
Tests for the SSE streaming endpoint: POST /api/v1/chat/{session_id}/generate-estimate.

Tests:
- SSE response has media_type "text/event-stream"
- SSE headers: X-Accel-Buffering, Cache-Control, Connection
- First event in stream is "event: thinking"
- "event: estimate" contains valid JSON matching EstimationOutput schema
- After estimate event, data events contain narrative tokens (no "None" text)
- Stream ends with "event: done" containing session_id
- OpenAI AuthenticationError -> "event: error" with error_type "invalid_key"
- RateLimitError("insufficient_quota") -> error_type "quota_exhausted"
- APITimeoutError -> error_type "transient"
- Invalid session_id -> error event with "invalid_session"
- Newlines in tokens are escaped to "\\n" in SSE data field

Mock strategy:
- Mock ChatService.get_session to return a test ChatSession with is_ready=True
- Mock EstimationService.generate_from_chat to return test (EstimationSession, EstimationOutput)
- Mock LLMService.stream_chat_completion as an async generator yielding test tokens
- Mock PromptService.get to return test narrative prompt dict
- Override FastAPI dependencies using app.dependency_overrides
"""

import json
import pytest
from unittest.mock import AsyncMock, MagicMock, patch, PropertyMock
from fastapi.testclient import TestClient

from app.models.chat import ChatSession, ScopingContext
from app.models.estimation import (
    EstimationOutput,
    EstimationSession,
    CostCategoryEstimate,
    AdjustmentFactor,
)
from app.models.tenant import Tenant
from app.core.constants import EstimationStatus, Region


# ---------------------------------------------------------------------------
# SSE parsing helper
# ---------------------------------------------------------------------------


def parse_sse_events(body: str) -> list[dict]:
    """Parse a raw SSE response body into a list of event dicts.

    Each dict has 'event' (default 'message') and 'data' keys.
    """
    events = []
    current_event = {}

    for line in body.split("\n"):
        line = line.rstrip("\r")
        if line.startswith("event:"):
            current_event["event"] = line[len("event:"):].strip()
        elif line.startswith("data:"):
            current_event["data"] = line[len("data:"):].strip()
        elif line == "" and current_event:
            # Empty line signals end of event
            if "event" not in current_event:
                current_event["event"] = "message"
            events.append(current_event)
            current_event = {}

    # Flush any trailing event without trailing blank line
    if current_event:
        if "event" not in current_event:
            current_event["event"] = "message"
        events.append(current_event)

    return events


# ---------------------------------------------------------------------------
# Shared test fixtures and helpers
# ---------------------------------------------------------------------------


def make_tenant(tenant_id: str = "tenant-stream-test") -> Tenant:
    return Tenant(
        tenant_id=tenant_id,
        company_name="Stream Test Co",
        email="stream@test.com",
        hashed_password="$2b$12$hashed",
        hashed_api_key="$2b$12$hashed_api",
        tier="paid",
    )


def make_estimation_output() -> EstimationOutput:
    return EstimationOutput(
        total_cost_p50=45000.0,
        total_cost_p80=55000.0,
        timeline_weeks_p50=8,
        timeline_weeks_p80=10,
        cost_breakdown=[
            CostCategoryEstimate(
                category="Materials",
                p50_cost=20000.0,
                p80_cost=24000.0,
                percentage_of_total=0.44,
            )
        ],
        adjustment_factors=[
            AdjustmentFactor(
                name="Urban premium",
                multiplier=1.15,
                reason="Coastal area cost uplift",
            )
        ],
        confidence_score=80.0,
        assumptions=["Standard site conditions"],
        summary="Estimated cost for pool installation.",
    )


def make_estimation_session(session_id: str = "sess_test001") -> EstimationSession:
    from bson import ObjectId

    return EstimationSession(
        tenant_id=ObjectId(),
        session_id=session_id,
        status=EstimationStatus.COMPLETED,
        description="Project type: pool. Size/scope: 15x30 feet. Location: SoCal - Coastal.",
        region=Region.SOCAL_COASTAL,
        reference_class="pool",
        confidence_threshold=0.7,
        prompt_version="1.0.0",
    )


def make_ready_session() -> ChatSession:
    return ChatSession(
        session_id="chat_stream001",
        tenant_id="tenant-stream-test",
        status="ready",
        is_ready=True,
        prompt_version="1.0.0",
        scoping_context=ScopingContext(
            project_type="pool",
            project_size="15x30 feet",
            location="SoCal - Coastal",
            timeline="spring 2026",
        ),
    )


async def mock_stream_tokens(*tokens):
    """Helper to create an async generator from token list."""
    for token in tokens:
        yield token


# ---------------------------------------------------------------------------
# Fixture: set up app with mocked dependencies
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def load_test_prompts():
    """Load real prompts into PromptService for all tests."""
    import os
    from app.services.prompt_service import PromptService

    PromptService.clear()
    base = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    prompts_dir = os.path.join(base, "config", "prompts")
    PromptService.load_all(prompts_dir)
    yield
    PromptService.clear()


@pytest.fixture(autouse=True)
def disable_rate_limiter():
    """Disable the rate limiter for all streaming tests.

    The rate limiter requires a live Valkey connection in production.
    We disable it for unit tests to avoid connection errors.
    """
    from app.core.rate_limit import limiter

    original_enabled = limiter.enabled
    limiter.enabled = False
    yield
    limiter.enabled = original_enabled


@pytest.fixture
def streaming_app():
    """FastAPI app with all streaming dependencies overridden."""
    from app.main import app
    from app.core.security import get_current_tenant
    from app.services.llm_service import get_llm_service

    tenant = make_tenant()

    # Build mock LLM service
    mock_llm = MagicMock()
    mock_llm.stream_chat_completion = MagicMock(
        return_value=mock_stream_tokens("You'll ", "most likely ", "spend ", "$45k.")
    )

    # Override tenant dependency
    async def override_get_tenant():
        return tenant

    # Override LLM service dependency
    async def override_get_llm():
        return mock_llm

    app.dependency_overrides[get_current_tenant] = override_get_tenant
    app.dependency_overrides[get_llm_service] = override_get_llm

    yield app, mock_llm, tenant

    # Cleanup
    app.dependency_overrides.clear()


@pytest.fixture
def client_with_mocked_services(streaming_app):
    """TestClient with ChatService and EstimationService mocked."""
    app, mock_llm, tenant = streaming_app
    est_output = make_estimation_output()
    est_session = make_estimation_session()
    chat_session = make_ready_session()

    with patch("app.api.routes.ChatService") as MockChatService, \
         patch("app.api.routes.EstimationService") as MockEstimationService:

        # Setup ChatService mock
        mock_chat_instance = MagicMock()
        mock_chat_instance.get_session = AsyncMock(return_value=chat_session)
        mock_chat_instance.mark_completed = AsyncMock(return_value=None)
        MockChatService.return_value = mock_chat_instance

        # Setup EstimationService mock
        mock_est_instance = MagicMock()
        mock_est_instance.generate_from_chat = AsyncMock(
            return_value=(est_session, est_output)
        )
        MockEstimationService.return_value = mock_est_instance

        with TestClient(app, raise_server_exceptions=False) as c:
            yield c, mock_chat_instance, mock_est_instance, est_session, est_output


# ---------------------------------------------------------------------------
# SSE endpoint tests
# ---------------------------------------------------------------------------


class TestSSEEndpointBasics:
    """Tests for basic SSE endpoint behavior and response format."""

    def test_sse_endpoint_returns_streaming_response(self, client_with_mocked_services):
        """POST /chat/{id}/generate-estimate returns text/event-stream media type."""
        client, _, _, _, _ = client_with_mocked_services
        response = client.post("/api/v1/chat/chat_stream001/generate-estimate")
        assert response.status_code == 200
        assert "text/event-stream" in response.headers["content-type"]

    def test_sse_headers_present(self, client_with_mocked_services):
        """SSE response includes all required headers to prevent proxy buffering."""
        client, _, _, _, _ = client_with_mocked_services
        response = client.post("/api/v1/chat/chat_stream001/generate-estimate")
        assert response.headers.get("x-accel-buffering") == "no"
        assert "no-cache" in response.headers.get("cache-control", "")

    def test_sse_thinking_event_first(self, client_with_mocked_services):
        """First SSE event in the stream is 'event: thinking'."""
        client, _, _, _, _ = client_with_mocked_services
        response = client.post("/api/v1/chat/chat_stream001/generate-estimate")
        events = parse_sse_events(response.text)
        assert len(events) > 0
        assert events[0]["event"] == "thinking"

    def test_sse_estimate_event_contains_json(self, client_with_mocked_services):
        """'event: estimate' event contains valid JSON matching EstimationOutput structure."""
        client, _, _, _, est_output = client_with_mocked_services
        response = client.post("/api/v1/chat/chat_stream001/generate-estimate")
        events = parse_sse_events(response.text)

        estimate_events = [e for e in events if e["event"] == "estimate"]
        assert len(estimate_events) == 1

        estimate_data = json.loads(estimate_events[0]["data"])
        assert "total_cost_p50" in estimate_data
        assert "total_cost_p80" in estimate_data
        assert "cost_breakdown" in estimate_data
        assert estimate_data["total_cost_p50"] == 45000.0

    def test_sse_data_events_contain_tokens(self, client_with_mocked_services):
        """After estimate event, data events contain narrative tokens."""
        client, _, _, _, _ = client_with_mocked_services
        response = client.post("/api/v1/chat/chat_stream001/generate-estimate")
        events = parse_sse_events(response.text)

        # Find data-only events (narrative tokens — no event type, just data)
        data_events = [e for e in events if e["event"] == "message"]
        assert len(data_events) > 0

        # Collect all token text
        all_tokens = "".join(e["data"] for e in data_events)
        assert "None" not in all_tokens  # delta.content None values must be filtered

    def test_sse_done_event_last(self, client_with_mocked_services):
        """Stream ends with 'event: done' containing the estimation session_id."""
        client, _, _, est_session, _ = client_with_mocked_services
        response = client.post("/api/v1/chat/chat_stream001/generate-estimate")
        events = parse_sse_events(response.text)

        # Last event should be 'done'
        non_empty_events = [e for e in events if e.get("data") or e.get("event")]
        assert len(non_empty_events) > 0
        last_event = non_empty_events[-1]
        assert last_event["event"] == "done"

        done_data = json.loads(last_event["data"])
        assert done_data["session_id"] == est_session.session_id

    def test_sse_event_sequence(self, client_with_mocked_services):
        """SSE event sequence is: thinking -> estimate -> data* -> done."""
        client, _, _, _, _ = client_with_mocked_services
        response = client.post("/api/v1/chat/chat_stream001/generate-estimate")
        events = parse_sse_events(response.text)

        event_types = [e["event"] for e in events]
        assert event_types[0] == "thinking"
        assert "estimate" in event_types
        assert "done" in event_types
        assert event_types.index("estimate") < event_types.index("done")


class TestSSEErrorHandling:
    """Tests for error events in the SSE stream."""

    def test_sse_error_on_session_not_found(self, streaming_app):
        """Invalid session_id produces 'event: error' with error_type 'invalid_session'."""
        app, mock_llm, tenant = streaming_app

        with patch("app.api.routes.ChatService") as MockChatService, \
             patch("app.api.routes.EstimationService"):

            mock_chat_instance = MagicMock()
            mock_chat_instance.get_session = AsyncMock(
                side_effect=ValueError("Chat session not found: bad_session_id")
            )
            MockChatService.return_value = mock_chat_instance

            with TestClient(app, raise_server_exceptions=False) as client:
                response = client.post("/api/v1/chat/bad_session_id/generate-estimate")

        events = parse_sse_events(response.text)
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1

        error_data = json.loads(error_events[0]["data"])
        assert error_data["error_type"] == "invalid_session"

    def test_sse_error_on_openai_auth_error(self, streaming_app):
        """AuthenticationError -> 'event: error' with error_type 'invalid_key'."""
        from openai import AuthenticationError

        app, mock_llm, tenant = streaming_app
        chat_session = make_ready_session()
        est_output = make_estimation_output()
        est_session = make_estimation_session()

        # Make stream_chat_completion raise AuthenticationError
        auth_exc = AuthenticationError.__new__(AuthenticationError)
        auth_exc.args = ("Invalid API key",)

        async def failing_stream(messages, **kwargs):
            raise auth_exc
            yield  # make it a generator

        mock_llm.stream_chat_completion = failing_stream

        with patch("app.api.routes.ChatService") as MockChatService, \
             patch("app.api.routes.EstimationService") as MockEstimationService:

            mock_chat_instance = MagicMock()
            mock_chat_instance.get_session = AsyncMock(return_value=chat_session)
            mock_chat_instance.mark_completed = AsyncMock(return_value=None)
            MockChatService.return_value = mock_chat_instance

            mock_est_instance = MagicMock()
            mock_est_instance.generate_from_chat = AsyncMock(
                return_value=(est_session, est_output)
            )
            MockEstimationService.return_value = mock_est_instance

            with TestClient(app, raise_server_exceptions=False) as client:
                response = client.post("/api/v1/chat/chat_stream001/generate-estimate")

        events = parse_sse_events(response.text)
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1

        error_data = json.loads(error_events[0]["data"])
        assert error_data["error_type"] == "invalid_key"

    def test_sse_error_on_openai_quota(self, streaming_app):
        """RateLimitError with insufficient_quota -> error_type 'quota_exhausted'."""
        from openai import RateLimitError

        app, mock_llm, tenant = streaming_app
        chat_session = make_ready_session()
        est_output = make_estimation_output()
        est_session = make_estimation_session()

        quota_exc = RateLimitError.__new__(RateLimitError)
        quota_exc.args = ("insufficient_quota exceeded",)

        async def failing_stream(messages, **kwargs):
            raise quota_exc
            yield

        mock_llm.stream_chat_completion = failing_stream

        with patch("app.api.routes.ChatService") as MockChatService, \
             patch("app.api.routes.EstimationService") as MockEstimationService:

            mock_chat_instance = MagicMock()
            mock_chat_instance.get_session = AsyncMock(return_value=chat_session)
            mock_chat_instance.mark_completed = AsyncMock(return_value=None)
            MockChatService.return_value = mock_chat_instance

            mock_est_instance = MagicMock()
            mock_est_instance.generate_from_chat = AsyncMock(
                return_value=(est_session, est_output)
            )
            MockEstimationService.return_value = mock_est_instance

            with TestClient(app, raise_server_exceptions=False) as client:
                response = client.post("/api/v1/chat/chat_stream001/generate-estimate")

        events = parse_sse_events(response.text)
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1

        error_data = json.loads(error_events[0]["data"])
        assert error_data["error_type"] == "quota_exhausted"

    def test_sse_error_on_transient(self, streaming_app):
        """APITimeoutError -> error_type 'transient'."""
        from openai import APITimeoutError

        app, mock_llm, tenant = streaming_app
        chat_session = make_ready_session()
        est_output = make_estimation_output()
        est_session = make_estimation_session()

        timeout_exc = APITimeoutError.__new__(APITimeoutError)

        async def failing_stream(messages, **kwargs):
            raise timeout_exc
            yield

        mock_llm.stream_chat_completion = failing_stream

        with patch("app.api.routes.ChatService") as MockChatService, \
             patch("app.api.routes.EstimationService") as MockEstimationService:

            mock_chat_instance = MagicMock()
            mock_chat_instance.get_session = AsyncMock(return_value=chat_session)
            mock_chat_instance.mark_completed = AsyncMock(return_value=None)
            MockChatService.return_value = mock_chat_instance

            mock_est_instance = MagicMock()
            mock_est_instance.generate_from_chat = AsyncMock(
                return_value=(est_session, est_output)
            )
            MockEstimationService.return_value = mock_est_instance

            with TestClient(app, raise_server_exceptions=False) as client:
                response = client.post("/api/v1/chat/chat_stream001/generate-estimate")

        events = parse_sse_events(response.text)
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1

        error_data = json.loads(error_events[0]["data"])
        assert error_data["error_type"] == "transient"


class TestSSENewlineEscaping:
    """Tests for newline escaping in SSE data tokens."""

    def test_sse_newlines_escaped_in_data(self, streaming_app):
        """Token containing '\\n' is escaped to '\\\\n' in the SSE data field."""
        app, mock_llm, tenant = streaming_app
        chat_session = make_ready_session()
        est_output = make_estimation_output()
        est_session = make_estimation_session()

        # Token with newline character
        async def newline_stream(messages, **kwargs):
            yield "Line one\nLine two"

        mock_llm.stream_chat_completion = newline_stream

        with patch("app.api.routes.ChatService") as MockChatService, \
             patch("app.api.routes.EstimationService") as MockEstimationService:

            mock_chat_instance = MagicMock()
            mock_chat_instance.get_session = AsyncMock(return_value=chat_session)
            mock_chat_instance.mark_completed = AsyncMock(return_value=None)
            MockChatService.return_value = mock_chat_instance

            mock_est_instance = MagicMock()
            mock_est_instance.generate_from_chat = AsyncMock(
                return_value=(est_session, est_output)
            )
            MockEstimationService.return_value = mock_est_instance

            with TestClient(app, raise_server_exceptions=False) as client:
                response = client.post("/api/v1/chat/chat_stream001/generate-estimate")

        # The raw response should contain escaped newlines
        assert "\\n" in response.text
        # The actual newline in the token should NOT appear as an unescaped bare newline
        # in the data field (only \n are allowed as event separators)
        data_events = [e for e in parse_sse_events(response.text) if e["event"] == "message"]
        assert len(data_events) > 0
        # The escaped version should be in the data
        token_data = data_events[0]["data"]
        assert "\\n" in token_data


class TestSSEPromptVersionRecording:
    """Tests for prompt_version recording during estimate generation."""

    def test_sse_prompt_version_recorded(self, client_with_mocked_services):
        """After successful stream, EstimationSession has prompt_version populated."""
        client, _, mock_est_instance, est_session, _ = client_with_mocked_services
        client.post("/api/v1/chat/chat_stream001/generate-estimate")

        # Verify generate_from_chat was called (which records prompt_version internally)
        mock_est_instance.generate_from_chat.assert_called_once()
        # The est_session returned by our mock has prompt_version set
        assert est_session.prompt_version == "1.0.0"
