from typing import List, Optional

from .protocol import KnowledgeProvider


class KnowledgeAggregator(KnowledgeProvider):
    """
    This class aggregates multiple knowledge providers and allows searching across all of them.
    """

    def __init__(self, knowledge_providers: List[KnowledgeProvider] = []):
        self.knowledge_providers = knowledge_providers

    def add_provider(self, knowledge_provider: KnowledgeProvider):
        self.knowledge_providers.append(knowledge_provider)

    async def search_knowledge(self, scope: Optional[str], conversation: Optional[str], query: str) -> Optional[str]:
        # TODO: Each provider should also give some description of its knowledge to the llm
        results = ""
        for provider in self.knowledge_providers:
            results += "\n" + await provider.search_knowledge(scope, conversation, query)

        return results
