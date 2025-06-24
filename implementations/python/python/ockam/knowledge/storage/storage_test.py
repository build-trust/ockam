import pytest

from ockam.knowledge.search import TextPiece
from .protocol import Storage
from .in_memory import InMemory
from .. import Database


async def test_in_memory_searches():
    await _test_searches(InMemory())

async def test_database_searches():
    storage = Database.from_environment(
        return_none_if_env_missing = True
    )
    if storage:
        await storage.delete_all()
        await _test_searches(storage)
    else:
        pytest.skip("database not configured")

async def _test_searches(storage: Storage):
    await storage.store_text_piece("scope1", "doc1", "Document 1", [TextPiece("Hello world", [0.0, 1.0, 0.0])])
    await storage.store_text_piece("scope1", "doc2", "Document 2", [TextPiece("Goodbye world", [0.0, -1.0, 0.0])])

    hits = await storage.search_text("scope1", [0.0, 1.0, 0.0], max_results=10, max_distance=0.5)
    assert len(hits) == 1
    assert hits[0].document_id == "doc1"
    assert hits[0].document_name == "Document 1"
    assert hits[0].text_piece == "Hello world"
    assert hits[0].distance == 0.0

    hits = await storage.search_text("scope1", [0.0, 1.0, 0.0], max_results=10, max_distance=1.0)
    assert len(hits) == 2
    assert hits[1].distance == 1.0


async def test_in_memory_store_document():
    await _test_store_document(InMemory())

async def test_database_store_document():
    storage = Database.from_environment(
        return_none_if_env_missing = True
    )
    if storage:
        await storage.delete_all()
        await _test_store_document(storage)
    else:
        pytest.skip("database not configured")

async def _test_store_document(storage: Storage):
    await storage.store_document("knowledge1", "doc1", "Document 1", "This is a test document.")
    documents = await storage.documents("knowledge1")
    assert len(documents) == 1
    assert documents[0].id == "doc1"
    assert documents[0].name == "Document 1"
    assert documents[0].content == "This is a test document."

    # Test storing and retrieving multiple documents
    await storage.store_document("knowledge1", "doc2", "Document 2", "This is another test document.")
    documents = await storage.documents("knowledge1")
    assert len(documents) == 2
