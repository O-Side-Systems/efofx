"""
Shared enums for the efOfX platform.

These pure enums are consumed by both the estimation service and future
IT/dev verticals. They have no application-server dependencies.
"""

from enum import Enum


class EstimationStatus(str, Enum):
    """Status of estimation sessions."""

    INITIATED = "initiated"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    CANCELLED = "cancelled"
    EXPIRED = "expired"


class ReferenceClassCategory(str, Enum):
    """Categories for reference class classification."""

    RESIDENTIAL = "residential"
    COMMERCIAL = "commercial"
    INDUSTRIAL = "industrial"
    INFRASTRUCTURE = "infrastructure"
    LANDSCAPING = "landscaping"
    RENOVATION = "renovation"
    NEW_CONSTRUCTION = "new_construction"


class CostBreakdownCategory(str, Enum):
    """Categories for cost breakdown in estimates."""

    MATERIALS = "materials"
    LABOR = "labor"
    EQUIPMENT = "equipment"
    PERMITS = "permits"
    DESIGN = "design"
    CONTINGENCY = "contingency"
    PROFIT_MARGIN = "profit_margin"


class Region(str, Enum):
    """Geographic regions for estimation."""

    SOCAL_COASTAL = "SoCal - Coastal"
    SOCAL_INLAND = "SoCal - Inland"
    NORCAL_BAY_AREA = "NorCal - Bay Area"
    NORCAL_CENTRAL = "NorCal - Central"
    ARIZONA_PHOENIX = "Arizona - Phoenix"
    ARIZONA_TUCSON = "Arizona - Tucson"
    NEVADA_LAS_VEGAS = "Nevada - Las Vegas"
    NEVADA_RENO = "Nevada - Reno"
