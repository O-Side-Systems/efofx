"""
Configuration settings for efOfX Estimation Service.

This module defines all configuration settings using Pydantic BaseSettings
for environment variable management and validation.
"""

from pydantic_settings import BaseSettings
from pydantic import Field
from typing import List, Optional
import os


class Settings(BaseSettings):
    """Application settings with environment variable support."""

    # Application
    DEBUG: bool = Field(default=False, env="DEBUG")
    APP_NAME: str = Field(default="efOfX Estimation Service", env="APP_NAME")
    VERSION: str = Field(default="1.0.0", env="VERSION")
    ENVIRONMENT: str = Field(default="development", env="ENVIRONMENT")

    # Server
    HOST: str = Field(default="0.0.0.0", env="HOST")
    PORT: int = Field(default=8000, env="PORT")

    # Security
    SECRET_KEY: str = Field(..., env="SECRET_KEY")
    JWT_SECRET_KEY: str = Field(..., env="JWT_SECRET_KEY")
    JWT_ALGORITHM: str = Field(default="HS256", env="JWT_ALGORITHM")
    JWT_EXPIRATION_HOURS: int = Field(default=24, env="JWT_EXPIRATION_HOURS")
    ENCRYPTION_KEY: str = Field(..., env="ENCRYPTION_KEY")
    ALLOWED_HOSTS: List[str] = Field(default=["*"], env="ALLOWED_HOSTS")
    ALLOWED_ORIGINS: List[str] = Field(default=["*"], env="ALLOWED_ORIGINS")
    
    # Database
    MONGO_URI: str = Field(..., env="MONGO_URI")
    MONGO_DB_NAME: str = Field(default="efofx_estimate", env="MONGO_DB_NAME")
    
    # OpenAI
    OPENAI_API_KEY: Optional[str] = Field(default=None, env="OPENAI_API_KEY")
    OPENAI_MODEL: str = Field(default="gpt-4o-mini", env="OPENAI_MODEL")
    OPENAI_MAX_TOKENS: int = Field(default=4000, env="OPENAI_MAX_TOKENS")
    OPENAI_TEMPERATURE: float = Field(default=0.7, env="OPENAI_TEMPERATURE")
    
    # Estimation
    MAX_ESTIMATION_SESSIONS: int = Field(default=100, env="MAX_ESTIMATION_SESSIONS")
    SESSION_TIMEOUT_MINUTES: int = Field(default=30, env="SESSION_TIMEOUT_MINUTES")
    
    # File Upload
    MAX_FILE_SIZE: int = Field(default=10 * 1024 * 1024, env="MAX_FILE_SIZE")  # 10MB
    ALLOWED_IMAGE_TYPES: List[str] = Field(
        default=["image/jpeg", "image/png", "image/webp"], 
        env="ALLOWED_IMAGE_TYPES"
    )
    
    # Logging
    LOG_LEVEL: str = Field(default="INFO", env="LOG_LEVEL")
    LOG_FORMAT: str = Field(default="json", env="LOG_FORMAT")

    # Rate Limiting
    RATE_LIMIT_ENABLED: bool = Field(default=True, env="RATE_LIMIT_ENABLED")
    RATE_LIMIT_PER_MINUTE: int = Field(default=60, env="RATE_LIMIT_PER_MINUTE")

    # Error Tracking (Optional - Sentry removed but keeping config for backwards compatibility)
    SENTRY_DSN: Optional[str] = Field(default=None, env="SENTRY_DSN")
    SENTRY_ENVIRONMENT: Optional[str] = Field(default=None, env="SENTRY_ENVIRONMENT")
    SENTRY_TRACES_SAMPLE_RATE: Optional[float] = Field(default=None, env="SENTRY_TRACES_SAMPLE_RATE")

    class Config:
        env_file = ".env"
        case_sensitive = False


# Global settings instance
settings = Settings() 