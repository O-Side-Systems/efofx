"""
efofx-shared: Shared utilities for the efOfX platform.

Provides framework-agnostic utilities consumable by any efOfX service
without pulling in FastAPI, Motor, or estimation-domain code.
"""

__version__ = "0.1.0"

from efofx_shared.core.constants import (
    CostBreakdownCategory,
    EstimationStatus,
    ReferenceClassCategory,
    Region,
)
from efofx_shared.utils.crypto import (
    decrypt_openai_key,
    derive_tenant_fernet_key,
    encrypt_openai_key,
    mask_openai_key,
)

__all__ = [
    "__version__",
    # Enums
    "EstimationStatus",
    "ReferenceClassCategory",
    "CostBreakdownCategory",
    "Region",
    # Crypto
    "derive_tenant_fernet_key",
    "encrypt_openai_key",
    "decrypt_openai_key",
    "mask_openai_key",
]
