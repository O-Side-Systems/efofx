"""
Unit tests for FeedbackEmailService.

Covers:
- is_configured property returns correct boolean based on RESEND_API_KEY
- send_email returns None and logs warning when RESEND_API_KEY is None (unconfigured)
- send_email calls resend.Emails.send via threadpool with correct params (configured)
- send_email returns None without re-raising when resend.Emails.send raises (graceful degradation)
"""

import pytest
from unittest.mock import patch, MagicMock, AsyncMock


class TestFeedbackEmailServiceIsConfigured:
    """FeedbackEmailService.is_configured reflects RESEND_API_KEY presence."""

    def test_is_configured_false_when_api_key_none(self):
        """is_configured is False when RESEND_API_KEY is None."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = None
            # Re-import to pick up mocked settings in __init__
            from app.services.feedback_email_service import FeedbackEmailService
            svc = FeedbackEmailService()
            assert svc.is_configured is False

    def test_is_configured_true_when_api_key_set(self):
        """is_configured is True when RESEND_API_KEY has a value."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = "re_test_abc123"
            import resend as resend_module
            with patch.object(resend_module, "api_key", None):
                from app.services.feedback_email_service import FeedbackEmailService
                svc = FeedbackEmailService()
                assert svc.is_configured is True


class TestFeedbackEmailServiceUnconfigured:
    """send_email gracefully skips sending when RESEND_API_KEY is not configured."""

    async def test_send_email_returns_none_when_unconfigured(self, caplog):
        """send_email returns None when RESEND_API_KEY is None."""
        import logging
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = None
            from app.services.feedback_email_service import FeedbackEmailService
            svc = FeedbackEmailService()

            result = await svc.send_email(
                to_email="user@example.com",
                subject="Test Subject",
                html_body="<p>Hello</p>",
                magic_link_url="https://example.com/feedback/abc123",
            )

            assert result is None

    async def test_send_email_logs_warning_when_unconfigured(self):
        """send_email logs a warning with the magic link URL when unconfigured."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = None
            import app.services.feedback_email_service as svc_mod
            from app.services.feedback_email_service import FeedbackEmailService
            svc = FeedbackEmailService()

            with patch.object(svc_mod.logger, "warning") as mock_warning:
                await svc.send_email(
                    to_email="user@example.com",
                    subject="Test Subject",
                    html_body="<p>Hello</p>",
                    magic_link_url="https://example.com/feedback/abc123",
                )
                mock_warning.assert_called_once()
                # Warning message should mention the email address
                call_args = mock_warning.call_args
                assert "user@example.com" in str(call_args)

    async def test_send_email_logs_not_provided_when_no_magic_link(self):
        """send_email logs '(not provided)' when magic_link_url is None."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = None
            import app.services.feedback_email_service as svc_mod
            from app.services.feedback_email_service import FeedbackEmailService
            svc = FeedbackEmailService()

            with patch.object(svc_mod.logger, "warning") as mock_warning:
                await svc.send_email(
                    to_email="user@example.com",
                    subject="Test Subject",
                    html_body="<p>Hello</p>",
                )
                mock_warning.assert_called_once()
                call_args = str(mock_warning.call_args)
                assert "(not provided)" in call_args


class TestFeedbackEmailServiceConfigured:
    """send_email sends via Resend SDK when RESEND_API_KEY is configured."""

    async def test_send_email_calls_resend_with_correct_params(self):
        """send_email calls resend.Emails.send with correct from/to/subject/html."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = "re_test_abc123"
            import resend as resend_module
            mock_result = {"id": "msg_abc123"}

            with patch.object(resend_module, "api_key", None):
                with patch("app.services.feedback_email_service.run_in_threadpool", new_callable=AsyncMock) as mock_threadpool:
                    mock_threadpool.return_value = mock_result
                    from app.services.feedback_email_service import FeedbackEmailService
                    svc = FeedbackEmailService()

                    result = await svc.send_email(
                        to_email="user@example.com",
                        subject="Please share feedback",
                        html_body="<p>Click to rate</p>",
                        magic_link_url="https://example.com/feedback/abc123",
                    )

                    assert result == "msg_abc123"
                    mock_threadpool.assert_called_once()
                    # First arg to run_in_threadpool is resend.Emails.send
                    call_args = mock_threadpool.call_args
                    assert call_args[0][0] == resend_module.Emails.send
                    # Second arg is the params dict
                    params_sent = call_args[0][1]
                    assert params_sent["to"] == ["user@example.com"]
                    assert params_sent["subject"] == "Please share feedback"
                    assert params_sent["html"] == "<p>Click to rate</p>"
                    assert params_sent["from"] == "feedback@efofx.com"

    async def test_send_email_returns_resend_message_id(self):
        """send_email returns the Resend message ID on success."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = "re_test_abc123"
            import resend as resend_module
            mock_result = {"id": "unique_msg_xyz"}

            with patch.object(resend_module, "api_key", None):
                with patch("app.services.feedback_email_service.run_in_threadpool", new_callable=AsyncMock) as mock_threadpool:
                    mock_threadpool.return_value = mock_result
                    from app.services.feedback_email_service import FeedbackEmailService
                    svc = FeedbackEmailService()

                    result = await svc.send_email(
                        to_email="test@test.com",
                        subject="Test",
                        html_body="<p>Test</p>",
                    )
                    assert result == "unique_msg_xyz"


class TestFeedbackEmailServiceGracefulDegradation:
    """send_email catches exceptions and returns None — never re-raises."""

    async def test_send_email_returns_none_on_exception(self):
        """send_email returns None when resend.Emails.send raises any exception."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = "re_test_abc123"
            import resend as resend_module

            with patch.object(resend_module, "api_key", None):
                with patch("app.services.feedback_email_service.run_in_threadpool", new_callable=AsyncMock) as mock_threadpool:
                    mock_threadpool.side_effect = Exception("Resend API error: 429 Too Many Requests")
                    from app.services.feedback_email_service import FeedbackEmailService
                    svc = FeedbackEmailService()

                    # Must not raise
                    result = await svc.send_email(
                        to_email="user@example.com",
                        subject="Test",
                        html_body="<p>Test</p>",
                    )
                    assert result is None

    async def test_send_email_logs_error_on_exception(self):
        """send_email logs the error details when Resend raises."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = "re_test_abc123"
            import resend as resend_module
            import app.services.feedback_email_service as svc_mod

            with patch.object(resend_module, "api_key", None):
                with patch("app.services.feedback_email_service.run_in_threadpool", new_callable=AsyncMock) as mock_threadpool:
                    mock_threadpool.side_effect = Exception("Network timeout")
                    from app.services.feedback_email_service import FeedbackEmailService
                    svc = FeedbackEmailService()

                    with patch.object(svc_mod.logger, "error") as mock_error:
                        await svc.send_email(
                            to_email="user@example.com",
                            subject="Test",
                            html_body="<p>Test</p>",
                        )
                        mock_error.assert_called_once()
                        error_call = str(mock_error.call_args)
                        assert "user@example.com" in error_call

    async def test_send_email_does_not_reraise_on_exception(self):
        """Calling code is never broken by email send failures."""
        with patch("app.services.feedback_email_service.settings") as mock_settings:
            mock_settings.RESEND_API_KEY = "re_test_abc123"
            import resend as resend_module

            with patch.object(resend_module, "api_key", None):
                with patch("app.services.feedback_email_service.run_in_threadpool", new_callable=AsyncMock) as mock_threadpool:
                    mock_threadpool.side_effect = ConnectionError("Cannot connect to Resend")
                    from app.services.feedback_email_service import FeedbackEmailService
                    svc = FeedbackEmailService()

                    # This must complete without raising
                    try:
                        await svc.send_email(
                            to_email="user@example.com",
                            subject="Test",
                            html_body="<p>Test</p>",
                        )
                    except Exception as e:
                        pytest.fail(f"send_email re-raised exception: {e}")
