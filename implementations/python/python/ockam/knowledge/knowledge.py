import aiofiles
import aiohttp
from urllib.parse import urlparse
from typing import Optional, List

from .chunkers import Chunker, NaiveChunker
from .extractors import TextExtractor, create_extractor
from .protocol import KnowledgeProvider, Storage
from .in_memory import InMemory
from .search import TextPiece, SearchHit
from ..models import Model
from ..ockam_in_rust_for_python import debug


class KnowledgeAggregator(KnowledgeProvider):
    """
    This class aggregates multiple knowledge providers and allows searching across all of them.
    """

    def __init__(self, knowledge_providers: List[KnowledgeProvider]):
        self.knowledge_providers = knowledge_providers

    async def search(self, query: str) -> List[SearchHit]:
        results = []
        for provider in self.knowledge_providers:
            results.extend(await provider.search(query))
        return results


class Knowledge(KnowledgeProvider):
    """
    This class serves as a basic knowledge provider that can store and retrieve whole documents.
    All the documents within the specified knowledge are returned when a search is performed.
    A document name can be specified to retrieve only a specific document.
    """

    def __init__(
        self,
        name: str,
        storage: Storage = InMemory(),
        text_extractor: TextExtractor = None,
        document_name: Optional[str] = None,
    ):
        """
        Initializes a Knowledge instance.

        :param name: A string representing the unique name or identifier for the instance.
        :param storage: An optional instance of Storage, defaulting to InMemory, which
            defines the storage mechanism for the object.
        :param text_extractor: An optional instance of TextExtractor for extracting text,
            defaulting to None. If None, a default text extractor is created.
        :param document_name: An optional string specifying the name of a document to
            associate with the instance, defaulting to None.
        """
        if text_extractor is None:
            text_extractor = create_extractor()
        self.name = name
        self.storage = storage
        self.document_name = document_name
        self.text_extractor = text_extractor

    async def search(self, _query: str) -> List[SearchHit]:
        return await self.storage.documents(
            self.name,
            document_name=self.document_name,
        )

    async def add_document(self, document_name: str, document_url: str, content_type: Optional[str] = None):
        content = await download_url(document_url)
        whole_document = await self.text_extractor.extract_text(content, content_type)
        await self.storage.store_document(
            self.name,
            document_name,
            whole_document,
        )

    async def add_text(self, document_name: str, text: str):
        await self.storage.store_document(
            self.name,
            document_name,
            text,
        )


class SearchableKnowledge(KnowledgeProvider):
    def __init__(
        self,
        name: str,
        model: Model = Model("ollama/nomic-embed-text"),
        storage: Storage = InMemory(),
        text_extractor: TextExtractor = None,
        chunker: Chunker = NaiveChunker(),
        max_results: int = 10,
        max_distance: float = 0.2,
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
        if text_extractor is None:
            text_extractor = create_extractor()

        self.name = name
        self.model = model
        self.storage = storage
        self.text_extractor = text_extractor
        self.chunker = chunker
        self.max_results = max_results
        self.max_distance = max_distance

    async def add_text(self, document_name: str, text: str, content_type: Optional[str] = None):
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

        embeddings = await self.model.embeddings([query])
        hits = await self.storage.search_text(self.name, embeddings[0], self.max_results, self.max_distance)
        debug(f"Search results for query '{query}': {len(hits)} hits found in knowledge '{self.name}'")
        return hits


async def download_url(url: str) -> bytes:
    parsed_url = urlparse(url)
    match parsed_url.scheme:
        case "http" | "https":
            async with aiohttp.ClientSession() as session:
                async with session.get(url) as response:
                    response.raise_for_status()
                    return await response.read()
        case "file" | "":
            async with aiofiles.open(parsed_url.path, mode="rb") as f:
                return await f.read()
    raise ValueError(f"Unsupported URL scheme: {parsed_url.scheme}. Supported schemes are 'http', 'https', and 'file'.")
