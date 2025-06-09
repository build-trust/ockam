from enum import Enum
from typing import Dict, List, Optional

from .protocol import Document
from .providers import KnowledgeProvider, SearchableKnowledgeProvider, SimpleKnowledgeProvider
from .search import SearchHit, SearchResults
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
    ALWAYS_WHOLE = "always",
    SEARCHABLE_PIECES = "searchable",


class Memory:
    namespace: str
    providers: List[KnowledgeProvider]
    whole_provider: SimpleKnowledgeProvider
    searchable_provider: SearchableKnowledgeProvider
    search_results: SearchResults
    max_knowledge_size: int

    def __init__(
        self,
        namespace: str,
        model: Model = Model.default_embedding_model(),
        max_knowledge_size: int = 4096,
        providers: Optional[List[KnowledgeProvider]] = None,
    ):
        """
        Initializes a Knowledge instance.

        :param namespace: A string representing the unique namespace for the knowledge.
        :param max_knowledge_size: Maximum size of knowledge in characters.
        :param providers: A list of KnowledgeProvider instances to be used for searching.
        """
        self.namespace = namespace
        self.whole_provider = SimpleKnowledgeProvider(namespace)
        self.searchable_provider = SearchableKnowledgeProvider(namespace, model)
        self.providers = [self.whole_provider, self.searchable_provider]
        if providers:
            self.providers.extend(providers)
        self.search_results = SearchResults()
        self.max_knowledge_size = max_knowledge_size

    async def add_document(self, document: Document, retrieval: Retrieval = Retrieval.ALWAYS_WHOLE) -> None:
        """
        Add a document to the knowledge base.

        :param document: The document to add.
        :param retrieval: Retrieval strategy to use.
        :raises ValueError: If the retrieval type is unknown.
        """
        if retrieval == Retrieval.ALWAYS_WHOLE:
            await self.whole_provider.add_document(document)
        elif retrieval == Retrieval.SEARCHABLE_PIECES:
            await self.searchable_provider.add_document(document)
        else:
            raise ValueError(f"Unknown retrieval type: {retrieval}")

    async def add_query(
        self, query: str,
    ) -> None:
        """
        This method will search across all providers and store the results.

        :param query: Search query string.
        """
        hits: List[SearchHit] = []
        for provider in self.providers:
            provider_hits = await provider.search(query)
            hits.extend(provider_hits)
        self.search_results.add(hits)

    def render_text(self) -> Optional[str]:
        """
        Renders the text from the search results into a string.

        :return: Formatted text from search results, or None when there are no results.
        """
        initial_knowledge_size = None
        while True:
            view: Dict[str, List[str]] = self.search_results.view()
            if len(view) > 0:
                contextual_knowledge: str = ""
                for document_name, text_pieces in view.items():
                    contextual_knowledge += f"Document name: {document_name}\n"
                    for text_piece in text_pieces:
                        contextual_knowledge += f"- {text_piece}\n"
                    contextual_knowledge += "\n"

                if len(contextual_knowledge) <= self.max_knowledge_size:
                    if initial_knowledge_size is not None:
                        logger.info(f"Knowledge size reduced from {initial_knowledge_size} to {len(contextual_knowledge)} characters.")
                    return contextual_knowledge
                else:
                    initial_knowledge_size = len(contextual_knowledge)
                    self.search_results.reduce_size()
            else:
                return None
