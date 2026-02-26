"""
Synthetic data generators for construction reference classes.

This package contains generators for creating realistic synthetic reference
classes across multiple construction project types and regions.
"""

from .pool import generate_pool_reference_classes
from .adu import generate_adu_reference_classes
from .kitchen import generate_kitchen_reference_classes
from .bathroom import generate_bathroom_reference_classes
from .landscaping import generate_landscaping_reference_classes
from .roofing import generate_roofing_reference_classes
from .flooring import generate_flooring_reference_classes

__all__ = [
    'generate_pool_reference_classes',
    'generate_adu_reference_classes',
    'generate_kitchen_reference_classes',
    'generate_bathroom_reference_classes',
    'generate_landscaping_reference_classes',
    'generate_roofing_reference_classes',
    'generate_flooring_reference_classes',
]
