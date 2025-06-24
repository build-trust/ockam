from enum import Enum
from typing import Dict, List, Optional, Any, Coroutine

from .protocol import Document, ProviderSearchResult
from .providers import KnowledgeProvider, SearchableKnowledgeProvider, SimpleKnowledgeProvider, \
    ProviderSearchResultAggregator
from ..models import Model
from ..logging.logging import get_logging_config
import logging.config

logging.config.dictConfig(get_logging_config())
logger = logging.getLogger("memory")


class Retrieval(Enum):
    """
    Strategies on how the document gets searched and retrieved.

    - ALWAYS_WHOLE:
        Retrieve the entire document as a single unit, regardless of the query.
        This strategy works well but can't be used when the document/s are too large to fit
        into the context window of the model.
    - SEARCHABLE_PIECES:
        Search within the document and retrieve only relevant sections matching the query.
        Use this for large or structured documents where only specific parts are needed.
    """
    WHOLE = "whole",
    SEARCHABLE = "searchable",

class Memory:
    scope: str
    knowledge_providers: List[KnowledgeProvider]
    whole_provider: SimpleKnowledgeProvider
    searchable_provider: SearchableKnowledgeProvider
    max_knowledge_size: int

    def __init__(
        self,
        scope: str,
        model: Model = Model.default_embedding_model(),
        max_knowledge_size: int = 4096,
        knowledge_providers: Optional[List[KnowledgeProvider]] = None,
    ):
        """
        Initializes a Knowledge instance.

        :param scope: A string representing the unique scope of the knowledge.
        :param max_knowledge_size: Maximum size of knowledge in characters.
        :param knowledge_providers: A list of KnowledgeProvider instances to be used for searching.
        """
        self.scope = scope
        self.whole_provider = SimpleKnowledgeProvider(scope)
        self.searchable_provider = SearchableKnowledgeProvider(scope, model)
        self.knowledge_providers = [self.whole_provider, self.searchable_provider]
        if knowledge_providers:
            self.knowledge_providers.extend(knowledge_providers)
        self.max_knowledge_size = max_knowledge_size

    async def add(self, document: Document, retrieval: Retrieval = Retrieval.WHOLE) -> None:
        """
        Add a document to the knowledge base.

        :param document: The document to add.
        :param retrieval: Retrieval strategy to use.
        :raises ValueError: If the retrieval type is unknown.
        """
        if retrieval == Retrieval.WHOLE:
            await self.whole_provider.add(document)
        elif retrieval == Retrieval.SEARCHABLE:
            await self.searchable_provider.add(document)
        else:
            raise ValueError(f"Unknown retrieval type: {retrieval}")

    async def search(
        self, query: str,
    ) -> ProviderSearchResultAggregator:
        """
        This method will search across all providers and store the results.

        :param query: Search query string.
        """
        results: List[ProviderSearchResult] = []
        for provider in self.knowledge_providers:
            results.append(
                await provider.search(query)
            )
        return ProviderSearchResultAggregator(results)
