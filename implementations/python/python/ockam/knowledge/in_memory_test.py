from .search import TextPiece
from .. import InMemory


async def test_searches():
    storage = InMemory()
    await storage.initialize()

    await storage.store_text_piece("knowledge1", "doc1", [TextPiece("Hello world", [0, 1, 0])])
    await storage.store_text_piece("knowledge1", "doc2", [TextPiece("Goodbye world", [0, -1, 0])])

    hits = await storage.search_text("knowledge1", [0, 1, 0], max_results=10, max_distance=0.5)
    assert len(hits) == 1
    assert hits[0].document_name == "doc1"
    assert hits[0].text_piece == "Hello world"
    assert hits[0].distance == 0.0

    hits = await storage.search_text("knowledge1", [0, 1, 0], max_results=10, max_distance=1.0)
    assert len(hits) == 2
    assert hits[1].distance == 1.0


async def test_store_document():
    storage = InMemory()
    await storage.initialize()

    await storage.store_document("knowledge1", "doc1", "This is a test document.")
    documents = await storage.documents("knowledge1")
    assert len(documents) == 1
    assert documents[0].text_piece == "This is a test document."

    # Test storing and retrieving multiple documents
    await storage.store_document("knowledge1", "doc2", "This is another test document.")
    documents = await storage.documents("knowledge1")
    assert len(documents) == 2
