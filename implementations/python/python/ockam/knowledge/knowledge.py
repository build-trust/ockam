from typing import Optional

from .protocol import KnowledgeProvider, Storage
from .searchable import SearchableKnowledge
from .unsearchable import UnsearchableKnowledge
from ..models.model import Model
from .chunkers.naive import NaiveChunker
from .chunkers import Chunker
from .extractors import TextExtractor
from .in_memory import InMemory
from ..logging.logging import InfoContext


class Knowledge(KnowledgeProvider, InfoContext):
    _logger = None

    @classmethod
    def class_logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ..logging.logging import get_logger

            cls._logger = get_logger("knowledge")
            return cls._logger

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
        self.logger = Knowledge.class_logger()
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
        with self.info(f"Adding document: '{document_name}' at: '{document_url}'",
                f"Added document: '{document_name}' at: '{document_url}'"):
            return await self.knowledge.add_document(document_name, document_url, content_type)

    async def add_text(self, document_name: str, text: str, content_type: Optional[str] = None):
        text_type = f"({content_type})" if content_type else ""
        with self.info(f"Adding document text: '{document_name}' {text_type}",
                f"Added document text: '{document_name}' {text_type}"):
            return await self.knowledge.add_text(document_name, text, content_type)

    async def search_knowledge(self, scope: Optional[str], conversation: Optional[str], query: str) -> Optional[str]:
        self.logger.info(f"Searching knowledge with query: '{query}'")
        result = await self.knowledge.search_knowledge(scope, conversation, query)
        self.logger.debug(f"Found knowledge with query: '{query}'. The result is: {result}")
        self.logger.info(f"Found knowledge with query: '{query}'")
        return result
