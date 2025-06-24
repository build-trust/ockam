from typing import List, Optional, Any, Dict

import os
import json
import urllib.parse
import psycopg
from psycopg.rows import Row

from ..search import TextPiece, SearchHit
from .protocol import Storage

from ..protocol import Document

# Relevant database schema:
"""
CREATE EXTENSION IF NOT EXISTS vector;
CREATE TABLE IF NOT EXISTS document_pieces (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    scope TEXT,
    name TEXT,
    text TEXT,
    embedding vector NOT NULL
);
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    scope TEXT,
    name TEXT,
    text TEXT
);

ALTER TABLE document_pieces ENABLE ROW LEVEL SECURITY;
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;

CREATE POLICY document_pieces_policy ON document_pieces USING (tenant_id = current_user);
CREATE POLICY documents_policy ON documents USING (tenant_id = current_user);
"""


class Database(Storage):
    connection_url: str
    user: Optional[str]
    password: Optional[str]
    connection: Optional[psycopg.AsyncConnection]
    tenant_id: Optional[str]

    def __init__(self, connection_url: str, user: Optional[str] = None, password: Optional[str] = None):
        self.connection_url = connection_url
        self.user = user
        self.password = password
        self.connection = None
        self.tenant_id = None

    async def initialize(self) -> None:
        """
        Initialize the database connection, do nothing if the connection is already established.
        """

        if self.connection is not None:
            return

        if self.user and self.password:
            self.connection = await psycopg.AsyncConnection.connect(
                self.connection_url, user=self.user, password=self.password
            )
        else:
            self.connection = await psycopg.AsyncConnection.connect(self.connection_url)

        async with self.connection.cursor() as cursor:
            await cursor.execute("SELECT \"current_user\"()")
            result = await cursor.fetchone()
            if result is None:
                raise Exception("Failed to connect to the database. No current user found.")
            self.tenant_id = result[0]

    async def delete_all(self):
        """For testing purposes, delete all entries"""
        await self.initialize()

        async with self.connection.cursor() as cursor:
            await cursor.execute("DELETE FROM documents")
            await cursor.execute("DELETE FROM document_pieces")
            await self.connection.commit()

    async def store_document(self, scope: str, id: str, name: str, text: str) -> None:
        await self.initialize()

        async with self.connection.cursor() as cursor:
            await cursor.execute(
                "INSERT INTO documents (tenant_id, scope, id, name, text) VALUES (%s, %s, %s, %s, %s)",
                (self.tenant_id, scope, id, name, text),
            )
            await self.connection.commit()

    async def documents(self, scope: str, id: Optional[str] = None) -> List[Document]:
        await self.initialize()

        async with self.connection.cursor() as cursor:
            if id:
                await cursor.execute(
                    "SELECT id, name, text FROM documents WHERE tenant_id = %s AND scope = %s AND document_id = %s",
                    (self.tenant_id, scope, id),
                )
                result: Optional[Row] = await cursor.fetchone()
                if result:
                    return [Document(id=result[0], name=result[1], content=result[2], content_type="text/plain")]
                else:
                    raise Exception(f"Document {id} not found.")
            else:
                await cursor.execute("SELECT id, name, text FROM documents WHERE tenant_id = %s AND scope = %s", (self.tenant_id, scope,))
                results: List[Row] = await cursor.fetchall()
                return [Document(id=result[0], name=result[1], content=result[2], content_type="text/plain") for result in results]

    async def store_text_piece(self, scope: str, id: str, name: str, pieces: List[TextPiece]) -> None:
        await self.initialize()

        async with self.connection.cursor() as cursor:
            for piece in pieces:
                await cursor.execute(
                    "INSERT INTO document_pieces (tenant_id, scope, id, name, text, embedding) VALUES (%s, %s, %s, %s, %s, %s)",
                    (self.tenant_id, scope, id, name, piece.text, piece.embedding),
                )
            await self.connection.commit()

    async def search_text(self, scope: str, embedding: List[float], max_results: int, max_distance: float) -> List[SearchHit]:
        await self.initialize()

        async with self.connection.cursor() as cursor:
            await cursor.execute(
                """
                SELECT id, name, text, (embedding <=> %s::sparsevec) as distance FROM document_pieces
                WHERE tenant_id = %s AND scope = %s AND (embedding <=> %s::sparsevec) <= %s
                ORDER BY distance LIMIT %s
                """,
                (embedding, self.tenant_id, scope, embedding, max_distance * 2.0, max_results),
            )
            results = await cursor.fetchall()
            # returned cosine distance is in the range [0, 2]
            return [SearchHit(hit[0], hit[1], hit[2], hit[3] * 0.5) for hit in results]

    @staticmethod
    def from_environment(return_none_if_env_missing: bool = False) -> Optional['Database']:
        """
        Create a Database instance from environment variables.

        :param return_none_if_env_missing: When True and no environment variables are set,
               the method will return `None` instead of raising an exception.
        :return: Storage instance or None if `none_when_undefined` is set.
        :raises Exception: If the required environment variables are missing.
        """
        connection_string: Optional[str] = os.getenv("OCKAM_DATABASE_CONNECTION_URL")
        if connection_string is not None:
            database = Database(connection_string)
            return database

        instance: Optional[str] = os.getenv("OCKAM_DATABASE_INSTANCE")
        user: Optional[str] = os.getenv("OCKAM_DATABASE_USER")
        password: Optional[str] = os.getenv("OCKAM_DATABASE_PASSWORD")
        user_and_password: Optional[str] = os.getenv("OCKAM_DATABASE_USERNAME_AND_PASSWORD")

        if return_none_if_env_missing and instance is None:
            return None

        if instance and user and password and not user_and_password:
            # Case 1: We have separate instance, user, and password variables
            pass
        elif instance and not user and not password and user_and_password:
            # Case 2: We have instance and a JSON containing username and password
            try:
                parsed: Dict[str, Any] = json.loads(user_and_password)
            except json.JSONDecodeError:
                raise Exception(f"Expected a JSON object. Got: {user_and_password}")

            if "username" in parsed and "password" in parsed:
                user = parsed["username"]
                password = parsed["password"]
            else:
                raise Exception(
                    f'Expected the username and password as `{{"username":"pgadmin", "password":"12345"}}`. '
                    f"Got: {user_and_password}"
                )
        else:
            raise Exception(
                "Not enough information to construct the connection string. "
                + "Please provide either OCKAM_DATABASE_CONNECTION_URL or "
                + "OCKAM_DATABASE_INSTANCE, OCKAM_DATABASE_USER, OCKAM_DATABASE_PASSWORD or "
                + "OCKAM_DATABASE_INSTANCE and OCKAM_DATABASE_USERNAME_AND_PASSWORD"
            )

        # A password can contain special characters, so we need to encode it
        url_encoded_password = urllib.parse.quote(password, safe="")
        connection_string = f"postgres://{user}:{url_encoded_password}@{instance}"

        database = Database(connection_string)
        return database
