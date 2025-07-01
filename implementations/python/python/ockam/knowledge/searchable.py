from typing import Optional, List

from .chunkers import Chunker, NaiveChunker
from .extractors import TextExtractor, create_extractor
from .protocol import KnowledgeProvider, Storage
from .in_memory import InMemory
from .search import TextPiece, SearchHit, SearchResults
from .util import download_url
from ..models import Model


class SearchableKnowledge(KnowledgeProvider):
    _logger = None

    @classmethod
    def class_logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ..logging.logging import get_logger

            cls._logger = get_logger("searchable_knowledge")
            return cls._logger

    def __init__(
            self,
            name: str,
            model: Model = None,
            storage: Storage = InMemory(),
            text_extractor: TextExtractor = None,
            chunker: Chunker = NaiveChunker(),
            max_results: int = 10,
            max_distance: float = 0.2,
            max_knowledge_size: int = 4096,
    ):
        """
        This class allows to store and search for text documents using vector search.

        :param name: The name of the system instance.
        :type name: str
        :param model: The model being utilized for processing operations.
        :type model: Model
        :param storage: Mechanism to store data. Defaults to an in-memory storage.
        :type storage: Storage
        :param text_extractor: An optional text extractor to process text from documents.
        If not provided, a default extractor will be created based on available libraries.
        :type text_extractor: TextExtractor
        :param chunker: The chunker to use for partitioning text into smaller pieces.
        :param max_results: Limit for the maximum number of results. Defaults to 10.
        :type max_results: int
        :param max_distance: Maximum allowable distance for operations. Defaults to 0.2.
        :type max_distance: float
        """
        self.logger = SearchableKnowledge.class_logger()
        # the InfoContext.info method is added at runtime to the instance because it cannot be added to the class
        # due to a cyclic import issue.
        from ..logging.logging import InfoContext

        self.info = InfoContext.info.__get__(self)
        if model is None:
            model = Model("ollama/nomic-embed-text")

        if text_extractor is None:
            text_extractor = create_extractor()

        self.name = name
        self.model = model
        self.storage = storage
        self.text_extractor = text_extractor
        self.chunker = chunker
        self.max_results = max_results
        self.max_distance = max_distance
        self.search_results = SearchResults()
        self.max_knowledge_size = max_knowledge_size

    async def add_text(self, document_name: str, text: str, content_type: Optional[str] = None):
        text_type = f"({content_type})" if content_type else ""
        with self.info(
                f"Adding document text: '{document_name}' {text_type}",
                f"Added document text: '{document_name}' {text_type}",
        ):
            whole_document = await self.text_extractor.extract_text(text, content_type)
            text_pieces = self.chunker.chunk(whole_document)

            # A single call is much faster than calling the model for each text piece
            embeddings = await self.model.embeddings(text_pieces)

            text_pieces = [TextPiece(text, embedding) for text, embedding in zip(text_pieces, embeddings)]
            await self.storage.store_text_piece(
                self.name,
                document_name,
                text_pieces,
            )

    async def add_document(self, document_name: str, document_url: str, content_type: Optional[str] = None):
        """
        Read a document using the unstructured library and add it to the knowledge base.

        :param content_type: The content type of the document.
        If not provided, it will be inferred.
        :param document_name: A name to identify the document in the knowledge base
        :type document_name: str
        :param document_url: Url to the document to be processed
        :type document_url: str
        """
        with self.info(
                f"Adding document: '{document_name}' at: '{document_url}'",
                f"Added document: '{document_name}' at: '{document_url}'",
        ):
            content = await download_url(document_url)
            whole_document = await self.text_extractor.extract_text(content, content_type)
            text_pieces = self.chunker.chunk(whole_document)

            # A single call is much faster than calling the model for each text piece
            embeddings = await self.model.embeddings(text_pieces)

            text_pieces = [TextPiece(text, embedding) for text, embedding in zip(text_pieces, embeddings)]
            await self.storage.store_text_piece(
                self.name,
                document_name,
                text_pieces,
            )

    async def search(self, query: str) -> List[SearchHit]:
        """
        Asynchronously searches for results that are most relevant to the provided query.
        It uses an embedding model to convert the query into an embedding vector for efficient
        searching within the backend storage. The results are then filtered based on the given
        maximum number of results and maximum distance.

        :param query: The query string to search for.
        :type query: str
        :return: A list of search results that match the criteria.
        :rtype: list
        """
        with self.info(f"Searching knowledge with query: '{query}'", f"Retrieved knowledge with query: '{query}'"):
            embeddings = await self.model.embeddings([query])
            hits = await self.storage.search_text(self.name, embeddings[0], self.max_results, self.max_distance)
            self.logger.debug(f"Search results for query '{query}': {len(hits)} hits found in knowledge '{self.name}'")
            return hits

    async def search_knowledge(self, _scope: Optional[str], conversation: Optional[str], query: str) -> Optional[str]:
        if not query or len(query) == 0:
            return None

        hits = await self.search(query)
        # TODO: Should it be cleared here at some point?
        self.search_results.add(hits)
        while True:
            view = self.search_results.view()
            if len(view) > 0:
                contextual_knowledge = ""
                for document_name, text_pieces in view.items():
                    contextual_knowledge += f"Document name: {document_name}\n"
                    for text_piece in text_pieces:
                        contextual_knowledge += f"- {text_piece}\n"
                    contextual_knowledge += "\n"
                if len(contextual_knowledge) <= self.max_knowledge_size:
                    break
            else:
                contextual_knowledge = None
                break
            self.search_results.reduce_size()

        return contextual_knowledge
