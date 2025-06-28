from typing import Optional


from .protocol import KnowledgeProvider, Storage
from .searchable import SearchableKnowledge
from .unsearchable import UnsearchableKnowledge
from ..models.model import Model
from .chunkers.naive import NaiveChunker
from .chunkers import Chunker
from .extractors import TextExtractor
from .in_memory import InMemory


class Knowledge(KnowledgeProvider):
    def __init__(
        self,
        name: str,
        searchable=False,
        # shared
        storage: Storage = InMemory(),
        text_extractor: TextExtractor = None,
        max_knowledge_size: int = 4096,
        # searchable-specific
        model: Model = None,
        chunker: Chunker = NaiveChunker(),
        max_results: int = 10,
        max_distance: float = 0.2,
        # unsearchable-specific
        document_name: Optional[str] = None,
    ):
        self.searchable = searchable

        if self.searchable:
            self.knowledge = SearchableKnowledge(
                name,
                storage=storage,
                text_extractor=text_extractor,
                max_knowledge_size=max_knowledge_size,
                model=model,
                chunker=chunker,
                max_results=max_results,
                max_distance=max_distance,
            )
        else:
            self.knowledge = UnsearchableKnowledge(
                name,
                storage=storage,
                text_extractor=text_extractor,
                max_knowledge_size=max_knowledge_size,
                document_name=document_name,
            )

    async def add_document(self, document_name: str, document_url: str, content_type: Optional[str] = None):
        return await self.knowledge.add_document(document_name, document_url, content_type)

    async def add_text(self, document_name: str, text: str, content_type: Optional[str] = None):
        return await self.knowledge.add_text(document_name, text, content_type)

    async def search_knowledge(self, scope: Optional[str], conversation: Optional[str], query: str) -> Optional[str]:
        return await self.knowledge.search_knowledge(scope, conversation, query)
