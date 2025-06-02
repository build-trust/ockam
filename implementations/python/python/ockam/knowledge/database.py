from typing import List

import os
import json
import urllib.parse
import psycopg

from .search import TextPiece, SearchHit
from .protocol import Storage

# Relevant database schema:
# CREATE EXTENSION IF NOT EXISTS vector;
# CREATE TABLE IF NOT EXISTS pieces (
#   id SERIAL PRIMARY KEY,
#   knowledge TEXT,
#   document_name TEXT,
#   text TEXT,
#   embedding vector NOT NULL
# )
# CREATE TABLE IF NOT EXISTS documents (
#   id SERIAL PRIMARY KEY,
#   knowledge TEXT,
#   document_name TEXT,
#   text TEXT
# )


class Database(Storage):
    def __init__(self, connection_url: str, user: str = None, password: str = None):
        self.connection_url = connection_url
        self.user = user
        self.password = password
        self.connection = None

    async def initialize(self):
        if self.connection is not None:
            return

        if self.user and self.password:
            self.connection = await psycopg.AsyncConnection.connect(
                self.connection_url, user=self.user, password=self.password
            )
        else:
            self.connection = await psycopg.AsyncConnection.connect(self.connection_url)

    async def store_document(self, knowledge: str, document_name: str, text: str):
        async with self.connection.cursor() as cursor:
            await cursor.execute(
                "INSERT INTO documents (knowledge, document_name, text) VALUES (%s, %s, %s)",
                (knowledge, document_name, text),
            )
            await self.connection.commit()

    async def documents(self, knowledge: str, document_name: str = None) -> list[SearchHit]:
        async with self.connection.cursor() as cursor:
            if document_name:
                await cursor.execute(
                    "SELECT document_name, text FROM documents WHERE knowledge = %s AND document_name = %s",
                    (knowledge, document_name),
                )
                result = await cursor.fetchone()
                if result:
                    return [SearchHit(result[0], result[1])]
                else:
                    raise RuntimeError(f"Document {document_name} not found.")
            else:
                await cursor.execute("SELECT document_name, text FROM documents WHERE knowledge = %s", (knowledge,))
                results = await cursor.fetchall()
                return [SearchHit(result[0], result[1]) for result in results]

    async def store_text_piece(self, knowledge: str, document_name: str, pieces: List[TextPiece]):
        async with self.connection.cursor() as cursor:
            for piece in pieces:
                await cursor.execute(
                    "INSERT INTO pieces (knowledge, document_name, text, embedding) VALUES (%s, %s, %s, %s)",
                    (knowledge, document_name, piece.text, piece.embedding),
                )
            await self.connection.commit()

    async def search_text(self, knowledge: str, embedding: list, max: int, max_distance: float) -> List[SearchHit]:
        async with self.connection.cursor() as cursor:
            await cursor.execute(
                "SELECT document_name, text, (embedding <=> %s::sparsevec) as distance FROM pieces "
                "WHERE knowledge = %s AND (embedding <=> %s::sparsevec) < %s "
                "ORDER BY distance LIMIT %s",
                (embedding, knowledge, embedding, max_distance * 2.0, max),
            )
            results = await cursor.fetchall()
            # returned cosine distance is in the range [0, 2]
            return [SearchHit(hit[0], hit[1], hit[2] * 0.5) for hit in results]

    @staticmethod
    async def from_environment():
        connection_string = os.getenv("OCKAM_DATABASE_CONNECTION_URL")
        if connection_string is not None:
            database = Database(connection_string)
            await database.initialize()
            return database

        instance = os.getenv("OCKAM_DATABASE_INSTANCE")
        user = os.getenv("OCKAM_DATABASE_USER")
        password = os.getenv("OCKAM_DATABASE_PASSWORD")
        user_and_password = os.getenv("OCKAM_DATABASE_USERNAME_AND_PASSWORD")

        if instance and user and password and not user_and_password:
            # Case 1: We have separate instance, user, and password variables
            pass
        elif instance and not user and not password and user_and_password:
            # Case 2: We have instance and a JSON containing username and password
            try:
                parsed = json.loads(user_and_password)
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
        await database.initialize()
        return database
