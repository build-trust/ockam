from typing import Optional

from mem0 import AsyncMemory

from ..models.model import Model

from .protocol import KnowledgeProvider


def local_config(model: Model, embed_model: Model):
    return {
        "vector_store": {
            "provider": "inmemory",
        },
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


class Mem0Knowledge(KnowledgeProvider):
    def __init__(self, memory: AsyncMemory):
        self.memory = memory

    @classmethod
    async def create(cls, model: Model, embeddings_model: Model):
        memory = await AsyncMemory.from_config(local_config(model, embeddings_model))
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
