import os
from typing import Optional
from mem0 import AsyncMemory
from ..models.model import Model
from .protocol import KnowledgeProvider


def model_config(model: Model, embed_model: Model):
    return {
        "llm": {
            "provider": "ockam",
            "config": {
                "temperature": 0,
                "max_tokens": 2000,
                "ockam_model": model,
            },
        },
        "embedder": {
            "provider": "ockam",
            "config": {
                "ockam_model": embed_model,
            },
        },
    }


def storage_config(model: Model, embed_model: Model):
    if os.environ.get("OCKAM_DATABASE_INSTANCE"):
        db_instance = os.environ.get("OCKAM_DATABASE_INSTANCE")
        tenant_id = os.environ.get("OCKAM_DATABASE_USER")
        password = os.environ.get("OCKAM_DATABASE_PASSWORD")
        host_port, dbname = db_instance.split("/", 1)
        host, port = host_port.split(":", 1)

        config = {
            "vector_store": {
                "provider": "pgvector",
                "config": {
                    "user": tenant_id,
                    "password": password,
                    "host": host,
                    "port": port,
                    "dbname": dbname,
                    "collection_name": "mem0_conversation",
                    "embedding_model_dims": 1536,
                },
            },
        }
    else:
        config = {
            "vector_store": {
                "provider": "inmemory",
            }
        }

    config.update(model_config(model, embed_model))
    return config


class Mem0Knowledge(KnowledgeProvider):
    def __init__(self, memory: AsyncMemory):
        self.memory = memory

    @classmethod
    async def create(cls, model: Model, embeddings_model: Model):
        memory = await AsyncMemory.from_config(storage_config(model, embeddings_model))
        return cls(memory)

    async def add(self, scope: Optional[str], conversation: Optional[str], messages):
        if not scope:
            return None

        user_id = scope

        # TODO: use conversation

        return await self.memory.add(messages, user_id=user_id)

    async def search_knowledge(self, scope: Optional[str], conversation: Optional[str], query: str) -> Optional[str]:
        if not scope:
            return None

        user_id = scope

        # TODO: use conversation

        if not query:
            return None

        relevant_memories = await self.memory.search(query=query, user_id=user_id, limit=3)
        relevant_memories = relevant_memories.get("results", None)
        if relevant_memories:
            return "\n".join(f"- {entry['memory']}" for entry in relevant_memories)

        return None
