"""
Seed database with synthetic construction reference classes.

This script generates and inserts synthetic reference classes into MongoDB
for all 7 construction types across 4 California regions.
"""

import asyncio
import sys
from pathlib import Path

# Add parent directories to path to import app modules
sys.path.insert(0, str(Path(__file__).parent.parent / 'efofx-estimate'))

from app.db.mongodb import connect_to_mongo, close_mongo_connection, get_reference_classes_collection
from generators import (
    generate_pool_reference_classes,
    generate_adu_reference_classes,
    generate_kitchen_reference_classes,
    generate_bathroom_reference_classes,
    generate_landscaping_reference_classes,
    generate_roofing_reference_classes,
    generate_flooring_reference_classes,
)


async def seed_reference_classes():
    """Generate and insert all synthetic reference classes."""
    print("🌱 Seeding database with synthetic reference classes...\n")

    # Connect to database
    await connect_to_mongo()
    collection = get_reference_classes_collection()

    # Clear existing synthetic data
    result = await collection.delete_many({"is_synthetic": True})
    print(f"✓ Cleared {result.deleted_count} existing synthetic reference classes\n")

    # Generate all reference classes
    all_classes = []

    generators = [
        ("Pool", generate_pool_reference_classes),
        ("ADU", generate_adu_reference_classes),
        ("Kitchen", generate_kitchen_reference_classes),
        ("Bathroom", generate_bathroom_reference_classes),
        ("Landscaping", generate_landscaping_reference_classes),
        ("Roofing", generate_roofing_reference_classes),
        ("Flooring", generate_flooring_reference_classes),
    ]

    for name, generator_func in generators:
        classes = generator_func()
        all_classes.extend(classes)
        print(f"✓ Generated {len(classes):3d} {name} reference classes")

    print(f"\nTotal generated: {len(all_classes)} reference classes")

    # Insert into database
    print("\n📦 Inserting into MongoDB...")
    if all_classes:
        result = await collection.insert_many(all_classes)
        print(f"✓ Inserted {len(result.inserted_ids)} reference classes successfully")

        # Verify insertion
        count = await collection.count_documents({"is_synthetic": True})
        print(f"✓ Database now contains {count} synthetic reference classes")

        # Show summary by category
        print("\n📊 Summary by construction type:")
        pipeline = [
            {"$match": {"is_synthetic": True}},
            {"$group": {
                "_id": "$subcategory",
                "count": {"$sum": 1},
                "avg_cost_p50": {"$avg": "$cost_distribution.p50"}
            }},
            {"$sort": {"_id": 1}}
        ]
        summary = await collection.aggregate(pipeline).to_list(None)
        for item in summary:
            print(f"  {item['_id']:15s}: {item['count']:3d} classes, avg p50 cost: ${item['avg_cost_p50']:,.0f}")

    await close_mongo_connection()
    print("\n✅ Database seeding complete!")


if __name__ == "__main__":
    asyncio.run(seed_reference_classes())
