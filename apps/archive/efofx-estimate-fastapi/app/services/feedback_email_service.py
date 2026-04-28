"""
Feedback email service for efOfX Estimation Service.

Wraps Resend SDK for transactional feedback magic link emails.
Resend SDK is synchronous — always dispatch via run_in_threadpool
or BackgroundTasks to avoid blocking the async event loop.
"""

import logging
from typing import Optional

import resend
from starlette.concurrency import run_in_threadpool

from app.core.config import settings

logger = logging.getLogger(__name__)


class FeedbackEmailService:
    """Send feedback request emails via Resend transactional API.

    When RESEND_API_KEY is not configured (local dev), logs the email
    details and magic link URL to the console instead of sending.
    """

    SENDER = "feedback@efofx.com"

    def __init__(self) -> None:
        self._configured = bool(settings.RESEND_API_KEY)
        if self._configured:
            resend.api_key = settings.RESEND_API_KEY

    @property
    def is_configured(self) -> bool:
        return self._configured

    async def send_email(
        self,
        to_email: str,
        subject: str,
        html_body: str,
        magic_link_url: Optional[str] = None,
    ) -> Optional[str]:
        """Send an HTML email via Resend.

        Args:
            to_email: Recipient email address.
            subject: Email subject line.
            html_body: Full HTML content of the email.
            magic_link_url: Optional magic link URL for dev-mode logging.

        Returns:
            Resend message ID on success, or None if not configured / on error.

        Note: Resend SDK is synchronous. This method wraps it with
        run_in_threadpool to avoid blocking the async event loop.
        """
        if not self._configured:
            logger.warning(
                "RESEND_API_KEY not configured — skipping feedback email to %s. "
                "Magic link for dev testing: %s",
                to_email,
                magic_link_url or "(not provided)",
            )
            return None

        params: resend.Emails.SendParams = {
            "from": self.SENDER,
            "to": [to_email],
            "subject": subject,
            "html": html_body,
        }

        try:
            result = await run_in_threadpool(resend.Emails.send, params)
            logger.info(
                "Feedback email sent to %s (Resend ID: %s)",
                to_email,
                (
                    result.get("id")
                    if isinstance(result, dict)
                    else getattr(result, "id", "unknown")
                ),
            )
            return (
                result.get("id")
                if isinstance(result, dict)
                else getattr(result, "id", None)
            )
        except Exception as exc:
            logger.error("Failed to send feedback email to %s: %s", to_email, exc)
            # Email failure must not break the feedback request flow
            return None
