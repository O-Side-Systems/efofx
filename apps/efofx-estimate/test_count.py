import asyncio
from app.db.mongodb import connect_to_mongo, get_reference_classes_collection

async def main():
    await connect_to_mongo()
    count = await get_reference_classes_collection().count_documents({'is_synthetic': True})
    print(count)

if __name__ == "__main__":
    asyncio.run(main())
