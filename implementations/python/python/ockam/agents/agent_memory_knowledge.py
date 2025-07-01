from typing import Optional


from ..models.model import Model
from ..knowledge import Mem0Knowledge


class AgentMemoryKnowledge:
    def __init__(self, mem0_knowledge: Mem0Knowledge):
        self.mem0_knowledge = mem0_knowledge
        self.added_messages = []

    @staticmethod
    async def create(memory_model: Model, memory_embeddings_model: Model):
        mem0_knowledge = await Mem0Knowledge.create(memory_model, memory_embeddings_model)

        return AgentMemoryKnowledge(mem0_knowledge)

    async def add(self, scope: Optional[str], conversation: Optional[str], messages):
        new_messages = messages

        # for message in messages:
        #     # FIXME: Far from perfect, messages should be assigned unique ids
        #     if {"scope": scope, "conversation": conversation, "message": message} not in self.added_messages:
        #         new_messages.append(message)

        if not new_messages:
            return

        await self.mem0_knowledge.add(scope=scope, conversation=conversation, messages=new_messages)

        for message in new_messages:
            self.added_messages.append({"scope": scope, "conversation": conversation, "message": message})

    async def search(self, scope: Optional[str], conversation: Optional[str], query: Optional[str]) -> Optional[str]:
        if not query:
            return None
        return await self.mem0_knowledge.search_knowledge(scope=scope, conversation=conversation, query=query)
