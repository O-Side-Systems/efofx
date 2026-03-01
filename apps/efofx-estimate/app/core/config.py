"""
Configuration settings for efOfX Estimation Service.

This module defines all configuration settings using Pydantic BaseSettings
for environment variable management and validation.
"""

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict
from typing import List, Optional


class Settings(BaseSettings):
    """Application settings with environment variable support."""

    model_config = SettingsConfigDict(
        env_file=".env",
        case_sensitive=False,
        populate_by_name=True,
    )

    # Application
    DEBUG: bool = False
    APP_NAME: str = "efOfX Estimation Service"
    VERSION: str = "1.0.0"
    ENVIRONMENT: str = "development"

    # Server
    HOST: str = "0.0.0.0"
    PORT: int = 8000

    # Security
    SECRET_KEY: str
    JWT_SECRET_KEY: str
    JWT_ALGORITHM: str = "HS256"
    JWT_EXPIRATION_HOURS: int = 24
    ENCRYPTION_KEY: str
    MASTER_ENCRYPTION_KEY: str = Field(
        description="Fernet master key for BYOK per-tenant key encryption"
    )
    ALLOWED_HOSTS: List[str] = ["*"]
    ALLOWED_ORIGINS: List[str] = ["*"]

    # Database
    MONGO_URI: str
    MONGO_DB_NAME: str = "efofx_estimate"

    # OpenAI
    OPENAI_API_KEY: Optional[str] = None
    OPENAI_MODEL: str = "gpt-4o-mini"
    OPENAI_MAX_TOKENS: int = 4000
    OPENAI_TEMPERATURE: float = 0.7

    # Estimation
    MAX_ESTIMATION_SESSIONS: int = 100
    SESSION_TIMEOUT_MINUTES: int = 30

    # File Upload
    MAX_FILE_SIZE: int = 10 * 1024 * 1024  # 10MB
    ALLOWED_IMAGE_TYPES: List[str] = ["image/jpeg", "image/png", "image/webp"]

    # Logging
    LOG_LEVEL: str = "INFO"
    LOG_FORMAT: str = "json"

    # Rate Limiting
    RATE_LIMIT_ENABLED: bool = True
    RATE_LIMIT_PER_MINUTE: int = 60

    # Valkey / Redis (for rate limiting — used in plan 02-03)
    VALKEY_URL: str = "redis://localhost:6379"

    # SMTP (for email verification)
    SMTP_USERNAME: Optional[str] = None
    SMTP_PASSWORD: Optional[str] = None
    SMTP_SERVER: str = "localhost"
    SMTP_PORT: int = 587
    SMTP_FROM: str = "noreply@efofx.ai"

    # Email settings for widget consultation notifications (DEBT-04)
    # All optional — email is silently skipped if not configured
    MAIL_USERNAME: Optional[str] = None
    MAIL_PASSWORD: Optional[str] = None
    MAIL_FROM: Optional[str] = None
    MAIL_PORT: int = 587
    MAIL_SERVER: Optional[str] = None

    # Application base URL (for verification links in emails)
    APP_BASE_URL: str = "http://localhost:8000"

    # Error Tracking (Optional - Sentry removed but keeping config for backwards compatibility)
    SENTRY_DSN: Optional[str] = None
    SENTRY_ENVIRONMENT: Optional[str] = None
    SENTRY_TRACES_SAMPLE_RATE: Optional[float] = None


# Global settings instance
settings = Settings()
