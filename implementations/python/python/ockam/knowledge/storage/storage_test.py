import pytest
import os
import psycopg
from psycopg.rows import dict_row

from ockam.knowledge.search import TextPiece
from .protocol import Storage
from .in_memory import InMemory
from .. import Database, create_storage


@pytest.fixture
def setup_env(monkeypatch):
    monkeypatch.setenv(
        "OCKAM_DATABASE_INSTANCE", os.environ.get("OCKAM_DATABASE_INSTANCE", "localhost:5432/postgres")
    )
    monkeypatch.setenv("OCKAM_DATABASE_USER", "postgres")
    monkeypatch.setenv("OCKAM_DATABASE_PASSWORD", "")

    connection = psycopg.connect("postgresql://postgres@localhost:5432/postgres", row_factory=dict_row)

    with connection.cursor() as cur:
        cur.execute("""
                    CREATE EXTENSION IF NOT EXISTS vector;
                    CREATE TABLE IF NOT EXISTS document_piece
                    (
                        id        TEXT PRIMARY KEY,
                        tenant_id TEXT   NOT NULL,
                        scope     TEXT,
                        name      TEXT,
                        text      TEXT,
                        embedding vector NOT NULL
                    );

                    DELETE
                    FROM document_piece;

                    CREATE TABLE IF NOT EXISTS document
                    (
                        id        TEXT PRIMARY KEY,
                        tenant_id TEXT NOT NULL,
                        scope     TEXT,
                        name      TEXT,
                        text      TEXT
                    );
                    DELETE
                    FROM document;""")
        connection.commit()


async def test_in_memory_searches():
    await _test_searches(InMemory())


async def test_database_searches():
    storage = create_storage()
    await _test_searches(storage)


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
    storage = create_storage()
    await _test_store_document(storage)


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
