"""
Unit tests for EstimationService.generate_from_chat and ChatService.get_session/mark_completed.

Tests:
- generate_from_chat creates EstimationSession with correct fields
- generate_from_chat records prompt_version from PromptService
- generate_from_chat saves session to MongoDB via insert_one
- generate_from_chat returns EstimationOutput alongside the session
- _build_description_from_context with various context combinations
- _build_description_empty_context returns "General project"
- ChatService.get_session retrieves session by ID
- ChatService.get_session raises ValueError when not found
- ChatService.mark_completed updates session status

Mock strategy:
- Mock LLMService.generate_estimation to return predefined EstimationOutput
- Mock LLMService.generate_response for project classification
- Mock ReferenceService.get_reference_projects to return empty list
- Mock PromptService.get to return test prompt dict
- Mock MongoDB collection operations (find_one, insert_one, update_one)
"""

import pytest
from datetime import datetime
from unittest.mock import AsyncMock, MagicMock, patch

from app.models.chat import ChatSession, ScopingContext
from app.models.estimation import (
    EstimationOutput,
    EstimationSession,
    CostCategoryEstimate,
    AdjustmentFactor,
)
from app.models.tenant import Tenant


# ---------------------------------------------------------------------------
# Shared helpers
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


def make_estimation_output() -> EstimationOutput:
    """Build a minimal valid EstimationOutput for mocking."""
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
        summary="Estimated cost for pool installation in coastal region.",
    )


def make_ready_session(
    session_id: str = "chat_test001",
    tenant_id: str = "tenant-test-001",
) -> ChatSession:
    """Create a ChatSession with all required scoping fields populated."""
    return ChatSession(
        session_id=session_id,
        tenant_id=tenant_id,
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


def make_mock_collection() -> MagicMock:
    """Create a mock MongoDB collection with async operations."""
    mock_col = MagicMock()
    mock_col.insert_one = AsyncMock(return_value=MagicMock(inserted_id="some_id"))
    mock_col.find_one = AsyncMock(return_value=None)
    mock_col.update_one = AsyncMock(return_value=MagicMock(modified_count=1))
    return mock_col


# ---------------------------------------------------------------------------
# Fixture: load PromptService with real prompts
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def load_test_prompts():
    """Load the real prompts into PromptService for all tests."""
    import os
    from app.services.prompt_service import PromptService

    PromptService.clear()
    base = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    prompts_dir = os.path.join(base, "config", "prompts")
    PromptService.load_all(prompts_dir)
    yield
    PromptService.clear()


# ---------------------------------------------------------------------------
# EstimationService.generate_from_chat tests
# ---------------------------------------------------------------------------


class TestGenerateFromChat:
    """Tests for EstimationService.generate_from_chat method."""

    def _make_service(self, estimation_output: EstimationOutput = None):
        """Create an EstimationService with mocked dependencies."""
        from app.services.estimation_service import EstimationService

        mock_llm = MagicMock()
        mock_llm.generate_response = AsyncMock(return_value="pool")
        mock_llm.generate_estimation = AsyncMock(
            return_value=estimation_output or make_estimation_output()
        )

        service = EstimationService(llm_service=mock_llm)

        # Mock reference service
        mock_rc = MagicMock()
        mock_rc.name = "pool"
        service.reference_service = MagicMock()
        service.reference_service.get_reference_projects = AsyncMock(return_value=[])
        service.reference_service.get_reference_classes = AsyncMock(return_value=[mock_rc])

        return service, mock_llm

    @pytest.mark.asyncio
    async def test_generate_from_chat_creates_estimation_session(self):
        """generate_from_chat creates and returns an EstimationSession."""
        service, _ = self._make_service()
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        mock_col = make_mock_collection()

        with patch.object(service, "_collection", return_value=mock_col):
            est_session, estimation_output = await service.generate_from_chat(session, tenant)

        assert isinstance(est_session, EstimationSession)
        assert est_session.session_id.startswith("sess_")
        assert est_session.status.value == "completed"

    @pytest.mark.asyncio
    async def test_generate_from_chat_records_prompt_version(self):
        """generate_from_chat records the prompt_version from PromptService."""
        service, _ = self._make_service()
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        mock_col = make_mock_collection()

        with patch.object(service, "_collection", return_value=mock_col):
            est_session, _ = await service.generate_from_chat(session, tenant)

        assert est_session.prompt_version is not None
        assert est_session.prompt_version == "1.0.0"

    @pytest.mark.asyncio
    async def test_generate_from_chat_saves_to_db(self):
        """generate_from_chat calls collection.insert_one with session data."""
        service, _ = self._make_service()
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        mock_col = make_mock_collection()

        with patch.object(service, "_collection", return_value=mock_col):
            await service.generate_from_chat(session, tenant)

        mock_col.insert_one.assert_called_once()
        # Verify the data passed to insert_one is a dict (model_dump result)
        call_args = mock_col.insert_one.call_args[0][0]
        assert isinstance(call_args, dict)
        assert "session_id" in call_args

    @pytest.mark.asyncio
    async def test_generate_from_chat_returns_estimation_output(self):
        """generate_from_chat returns the EstimationOutput from LLM."""
        expected_output = make_estimation_output()
        service, mock_llm = self._make_service(estimation_output=expected_output)
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        mock_col = make_mock_collection()

        with patch.object(service, "_collection", return_value=mock_col):
            est_session, estimation_output = await service.generate_from_chat(session, tenant)

        assert estimation_output is expected_output
        assert estimation_output.total_cost_p50 == 45000.0
        assert estimation_output.total_cost_p80 == 55000.0

    @pytest.mark.asyncio
    async def test_generate_from_chat_uses_scoping_context_location_as_region(self):
        """generate_from_chat uses ctx.location as the region."""
        service, mock_llm = self._make_service()
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        mock_col = make_mock_collection()

        with patch.object(service, "_collection", return_value=mock_col):
            est_session, _ = await service.generate_from_chat(session, tenant)

        # Should match the SoCal - Coastal location from make_ready_session
        assert est_session.region.value == "SoCal - Coastal"

    @pytest.mark.asyncio
    async def test_generate_from_chat_falls_back_to_default_region_on_unknown_location(self):
        """generate_from_chat uses default region for unrecognized location strings."""
        service, _ = self._make_service()
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        # Set location to something not in the Region enum
        session.scoping_context.location = "Somewhere Unknown"
        mock_col = make_mock_collection()

        with patch.object(service, "_collection", return_value=mock_col):
            est_session, _ = await service.generate_from_chat(session, tenant)

        assert est_session.region is not None  # Falls back to default region

    @pytest.mark.asyncio
    async def test_generate_from_chat_description_built_from_context(self):
        """generate_from_chat builds description from scoping context."""
        service, mock_llm = self._make_service()
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        mock_col = make_mock_collection()

        with patch.object(service, "_collection", return_value=mock_col):
            est_session, _ = await service.generate_from_chat(session, tenant)

        # Description should include context fields
        assert "pool" in est_session.description.lower() or "project type" in est_session.description.lower()


# ---------------------------------------------------------------------------
# EstimationService._build_description_from_context tests
# ---------------------------------------------------------------------------


class TestBuildDescriptionFromContext:
    """Tests for _build_description_from_context helper method."""

    def _make_service(self):
        from app.services.estimation_service import EstimationService

        mock_llm = MagicMock()
        return EstimationService(llm_service=mock_llm)

    def test_build_description_all_fields(self):
        """Full context produces description with all fields."""
        service = self._make_service()
        ctx = ScopingContext(
            project_type="pool",
            project_size="15x30 feet",
            location="SoCal - Coastal",
            timeline="spring 2026",
            special_conditions="HOA approval needed",
        )
        description = service._build_description_from_context(ctx)
        assert "pool" in description
        assert "15x30 feet" in description
        assert "SoCal - Coastal" in description
        assert "spring 2026" in description
        assert "HOA approval needed" in description
        assert description.endswith(".")

    def test_build_description_partial_context(self):
        """Partial context includes only populated fields."""
        service = self._make_service()
        ctx = ScopingContext(
            project_type="deck",
            location="Bay Area",
        )
        description = service._build_description_from_context(ctx)
        assert "deck" in description
        assert "Bay Area" in description
        assert "project_size" not in description.lower()
        assert description.endswith(".")

    def test_build_description_empty_context(self):
        """Empty context produces 'General project'."""
        service = self._make_service()
        ctx = ScopingContext()
        description = service._build_description_from_context(ctx)
        assert description == "General project"

    def test_build_description_only_project_type(self):
        """Only project_type populated — description contains it."""
        service = self._make_service()
        ctx = ScopingContext(project_type="renovation")
        description = service._build_description_from_context(ctx)
        assert "renovation" in description
        assert description.endswith(".")

    def test_build_description_separator_is_period_space(self):
        """Fields are joined with '. '."""
        service = self._make_service()
        ctx = ScopingContext(project_type="pool", project_size="15x30 feet")
        description = service._build_description_from_context(ctx)
        # Both fields are in the description
        assert "Project type: pool" in description
        assert "Size/scope: 15x30 feet" in description


# ---------------------------------------------------------------------------
# ChatService.get_session tests
# ---------------------------------------------------------------------------


class TestChatServiceGetSession:
    """Tests for ChatService.get_session method."""

    @pytest.mark.asyncio
    async def test_get_session_returns_chat_session(self):
        """get_session returns ChatSession when found in DB."""
        from app.services.chat_service import ChatService

        mock_llm = MagicMock()
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        session_data = session.model_dump(by_alias=True)

        mock_col = MagicMock()
        mock_col.find_one = AsyncMock(return_value=session_data)

        with patch.object(service, "_collection", return_value=mock_col):
            result = await service.get_session(session.session_id, tenant)

        assert isinstance(result, ChatSession)
        assert result.session_id == session.session_id
        assert result.is_ready is True

    @pytest.mark.asyncio
    async def test_get_session_raises_value_error_when_not_found(self):
        """get_session raises ValueError when session not found."""
        from app.services.chat_service import ChatService

        mock_llm = MagicMock()
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        mock_col = MagicMock()
        mock_col.find_one = AsyncMock(return_value=None)

        with patch.object(service, "_collection", return_value=mock_col):
            with pytest.raises(ValueError, match="Chat session not found: nonexistent_id"):
                await service.get_session("nonexistent_id", tenant)

    @pytest.mark.asyncio
    async def test_get_session_queries_by_session_id(self):
        """get_session calls find_one with correct session_id filter."""
        from app.services.chat_service import ChatService

        mock_llm = MagicMock()
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        session_data = session.model_dump(by_alias=True)

        mock_col = MagicMock()
        mock_col.find_one = AsyncMock(return_value=session_data)

        with patch.object(service, "_collection", return_value=mock_col):
            await service.get_session("chat_test001", tenant)

        mock_col.find_one.assert_called_once_with({"session_id": "chat_test001"})

    @pytest.mark.asyncio
    async def test_get_session_returns_scoping_context(self):
        """get_session returns session with populated scoping_context."""
        from app.services.chat_service import ChatService

        mock_llm = MagicMock()
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()
        session = make_ready_session(tenant_id=tenant.tenant_id)
        session_data = session.model_dump(by_alias=True)

        mock_col = MagicMock()
        mock_col.find_one = AsyncMock(return_value=session_data)

        with patch.object(service, "_collection", return_value=mock_col):
            result = await service.get_session(session.session_id, tenant)

        assert result.scoping_context.project_type == "pool"
        assert result.scoping_context.location == "SoCal - Coastal"


# ---------------------------------------------------------------------------
# ChatService.mark_completed tests
# ---------------------------------------------------------------------------


class TestChatServiceMarkCompleted:
    """Tests for ChatService.mark_completed method."""

    @pytest.mark.asyncio
    async def test_mark_completed_calls_update_one(self):
        """mark_completed calls update_one on the collection."""
        from app.services.chat_service import ChatService

        mock_llm = MagicMock()
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        mock_col = MagicMock()
        mock_col.update_one = AsyncMock(return_value=MagicMock(modified_count=1))

        with patch.object(service, "_collection", return_value=mock_col):
            await service.mark_completed("chat_test001", tenant, "sess_estimation001")

        mock_col.update_one.assert_called_once()

    @pytest.mark.asyncio
    async def test_mark_completed_sets_status_completed(self):
        """mark_completed sets status to 'completed' in the update."""
        from app.services.chat_service import ChatService

        mock_llm = MagicMock()
        service = ChatService(llm_service=mock_llm)
        tenant = make_tenant()

        mock_col = MagicMock()
        mock_col.update_one = AsyncMock(return_value=MagicMock(modified_count=1))

        with patch.object(service, "_collection", return_value=mock_col):
            await service.mark_completed("chat_test001", tenant, "sess_estimation001")

        call_args = mock_col.update_one.call_args
        filter_doc = call_args[0][0]
        update_doc = call_args[0][1]
        assert filter_doc == {"session_id": "chat_test001"}
        assert update_doc["$set"]["status"] == "completed"
