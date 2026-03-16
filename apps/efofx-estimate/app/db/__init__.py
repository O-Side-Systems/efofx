"""
Database layer for efOfX Estimation Service.

This package contains all database-related functionality including
MongoDB connection management and data access patterns.
"""

from .mongodb import get_database, get_collection

__all__ = [
    "get_database",
    "get_collection",
]
