"""
PyObjectId utility for Pydantic v2 compatibility.

Legacy models that use MongoDB ObjectId as primary identifiers import this.
New models (Tenant) use UUID strings instead.
"""

from pydantic_core import core_schema
from bson import ObjectId


class PyObjectId(ObjectId):
    """Custom ObjectId for Pydantic v2 compatibility (used in legacy models)."""

    @classmethod
    def __get_pydantic_core_schema__(cls, source_type, handler):
        return core_schema.no_info_plain_validator_function(cls._validate)

    @classmethod
    def _validate(cls, v):
        if isinstance(v, ObjectId):
            return v
        if isinstance(v, str):
            if not ObjectId.is_valid(v):
                raise ValueError("Invalid ObjectId")
            return ObjectId(v)
        raise ValueError("Invalid ObjectId")
