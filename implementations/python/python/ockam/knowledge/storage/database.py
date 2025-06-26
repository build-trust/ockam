from typing import List, Optional, Any, Dict

import os
from os import environ
import json
from urllib.parse import quote
import psycopg
from psycopg.rows import Row, dict_row

from ..search import TextPiece, SearchHit
from .protocol import Storage

from ..protocol import Document


class Database(Storage):
    connection: Optional[psycopg.AsyncConnection]
    tenant_id: Optional[str]

    def __init__(self):
        self.connection = None
        self.tenant_id = None
        self.initialize()

    async def initialize(self) -> None:
        """
        Initialize the database connection from environment variables.
        """
        db_instance = environ.get("OCKAM_DATABASE_INSTANCE")
        self.tenant_id = environ.get("OCKAM_DATABASE_USER")
        password = environ.get("OCKAM_DATABASE_PASSWORD")

        connection_url = f"postgresql://{self.tenant_id}:{quote(password)}@{db_instance}"
        self.connection = psycopg.connect(connection_url, row_factory=dict_row)

    async def store_document(self, scope: str, id: str, name: str, text: str) -> None:
        async with self.connection.cursor() as cursor:
            await cursor.execute(
                "INSERT INTO document (tenant_id, scope, id, name, text) VALUES (%s, %s, %s, %s, %s)",
                (self.tenant_id, scope, id, name, text),
            )
            await self.connection.commit()

    async def documents(self, scope: str, id: Optional[str] = None) -> List[Document]:
        async with self.connection.cursor() as cursor:
            if id:
                await cursor.execute(
                    "SELECT id, name, text FROM document WHERE scope = %s AND document_id = %s",
                    (self.tenant_id, scope, id),
                )
                result: Optional[Row] = await cursor.fetchone()
                if result:
                    return [Document(id=result[0], name=result[1], content=result[2], content_type="text/plain")]
                else:
                    raise Exception(f"Document {id} not found.")
            else:
                await cursor.execute("SELECT id, name, text FROM document WHERE scope = %s",
                    (self.tenant_id, scope,))
                results: List[Row] = await cursor.fetchall()
                return [Document(id=result[0], name=result[1], content=result[2], content_type="text/plain") for result
                    in results]

    async def store_text_piece(self, scope: str, id: str, name: str, pieces: List[TextPiece]) -> None:
        await self.initialize()

        async with self.connection.cursor() as cursor:
            for piece in pieces:
                await cursor.execute(
                    "INSERT INTO document_piece (tenant_id, scope, id, name, text, embedding) VALUES (%s, %s, %s, %s, %s, %s)",
                    (self.tenant_id, scope, id, name, piece.text, piece.embedding),
                )
            await self.connection.commit()

    async def search_text(self, scope: str, embedding: List[float], max_results: int, max_distance: float) -> List[
        SearchHit]:
        await self.initialize()

        async with self.connection.cursor() as cursor:
            await cursor.execute(
                """
                SELECT id, name, text, (embedding <=> %s::sparsevec) as distance
                FROM document_piece
                WHERE scope = %s
                  AND (embedding <=> %s::sparsevec) <= %s
                ORDER BY distance
                LIMIT %s
                """,
                (embedding, self.tenant_id, scope, embedding, max_distance * 2.0, max_results),
            )
            results = await cursor.fetchall()
            # returned cosine distance is in the range [0, 2]
            return [SearchHit(hit[0], hit[1], hit[2], hit[3] * 0.5) for hit in results]
