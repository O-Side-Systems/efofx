"""
File utility functions for efOfX Estimation Service.

This module provides utilities for file handling, validation,
and upload management.
"""

import os
import uuid
import logging
from fastapi import UploadFile

from app.core.config import settings

logger = logging.getLogger(__name__)


def validate_file_type(file: UploadFile) -> bool:
    """Validate file type against allowed types."""
    return file.content_type in settings.ALLOWED_IMAGE_TYPES


def validate_file_size(file: UploadFile) -> bool:
    """Validate file size against maximum allowed size."""
    return file.size <= settings.MAX_FILE_SIZE


async def save_uploaded_file(file: UploadFile, session_id: str) -> str:
    """Save uploaded file and return file path."""
    try:
        # Create upload directory if it doesn't exist
        upload_dir = os.path.join(settings.UPLOAD_DIR, session_id)
        os.makedirs(upload_dir, exist_ok=True)

        # Generate unique filename
        file_extension = os.path.splitext(file.filename)[1]
        unique_filename = f"{uuid.uuid4().hex}{file_extension}"
        file_path = os.path.join(upload_dir, unique_filename)

        # Save file
        with open(file_path, "wb") as buffer:
            content = await file.read()
            buffer.write(content)

        # Return file URL (in production, this would be a cloud storage URL)
        return f"/uploads/{session_id}/{unique_filename}"

    except Exception as e:
        logger.error(f"Error saving uploaded file: {e}")
        raise


def get_file_extension(filename: str) -> str:
    """Get file extension from filename."""
    return os.path.splitext(filename)[1].lower()


def is_valid_image_extension(extension: str) -> bool:
    """Check if file extension is valid for images."""
    valid_extensions = [".jpg", ".jpeg", ".png", ".webp"]
    return extension.lower() in valid_extensions


def generate_file_url(session_id: str, filename: str) -> str:
    """Generate file URL for uploaded file."""
    return f"https://storage.efofx.ai/uploads/{session_id}/{filename}"
