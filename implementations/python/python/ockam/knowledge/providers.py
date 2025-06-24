import aiofiles
import aiohttp
from urllib.parse import urlparse
from typing import Optional, List, Dict

from .chunkers import Chunker, NaiveChunker
from .extractors import TextExtractor, create_extractor
from .protocol import KnowledgeProvider, Document, ProviderSearchResult
from .storage import Storage, create_storage
from .search import TextPiece, SearchHit
from ..models import Model
from ..ockam_in_rust_for_python import debug


class ProviderSearchResultAggregator(ProviderSearchResult):
    results: List[ProviderSearchResult]
    last_index: int = 0
    def __init__(self, results=None):
        if results is None:
            results = []
        self.results = results

    def __add__(self, other):
        if isinstance(other, ProviderSearchResultAggregator):
            return ProviderSearchResultAggregator(self.results + other.results)
        else:
            raise TypeError(f"Cannot add {type(other)} to ProviderSearchResultAggregator")
    def render(self) -> Optional[str]:
        aggregated_result = ""
        for result in self.results:
            rendered_result = result.render()
            if rendered_result:
                aggregated_result += f"{rendered_result}\n"
        return aggregated_result if aggregated_result else None

    def reduce_size(self) -> None:
        # reduce size using round-robin
        self.results[self.last_index].reduce_size()
        self.last_index += 1
        if self.last_index >= len(self.results):
            self.last_index = 0

class KnowledgeProviderAggregator(KnowledgeProvider):
    """
    This class aggregates multiple knowledge providers and allows searching across all of them.
    """

    def __init__(self, knowledge_providers: List[KnowledgeProvider]):
        self.knowledge_providers = knowledge_providers

    async def search(self, query: str) -> ProviderSearchResult:
        results = []
        for provider in self.knowledge_providers:
            result = await provider.search(query)
            if result:
                results.append(result)
        return ProviderSearchResultAggregator(results)


class SimpleKnowledgeProviderResult(ProviderSearchResult):
    def __init__(self, documents: List[Document]):
        self.documents = documents

    def render(self) -> Optional[str]:
        if not self.documents:
            return None
        result = "I've found the following documents in the knowledge base:\n"
        for document in self.documents:
            result += f""""
Document Name: {document.name}
DOCUMENT BEGIN
{document.content}
DOCUMENT END

"""
        return result

    def reduce_size(self) -> None:
        pass

class SimpleKnowledgeProvider(KnowledgeProvider):
    """
    This class serves as a basic knowledge provider that can store and retrieve whole documents.
    All the documents within the specified knowledge are returned when a search is performed.
    A document id can be specified to retrieve only a specific document.
    """

    def __init__(
        self,
        scope: str,
        storage: Optional[Storage] = None,
        text_extractor: TextExtractor = None,
        document_id: Optional[str] = None,
    ):
        """
        Initializes a Knowledge instance.

        :param scope: A string representing the unique name or identifier for the instance.
        :param storage: An optional instance of Storage, defaulting to InMemory, which
            defines the storage mechanism for the object.
        :param text_extractor: An optional instance of TextExtractor for extracting text,
            defaulting to None. If None, a default text extractor is created.
        :param document_id: An optional string specifying the name of a document to
            associate with the instance, defaulting to None.
        """

        if storage is None:
            storage = create_storage()

        if text_extractor is None:
            text_extractor = create_extractor()

        self.scope = scope
        self.storage = storage
        self.document_id = document_id
        self.text_extractor = text_extractor

    async def search(self, _query: str) -> SimpleKnowledgeProviderResult:
        return SimpleKnowledgeProviderResult(await self.storage.documents(
            self.scope,
            self.document_id,
        ))

    async def add(self, document: Document):
        if document.url is not None:
            content = await download_url(document.url)
            whole_document = await self.text_extractor.extract_text(content, document.content_type)
        else:
            whole_document = await self.text_extractor.extract_text(document.content, document.content_type)

        await self.storage.store_document(
            self.scope,
            document.id,
            document.name,
            whole_document,
        )
class SearchableKnowledgeProviderResult(ProviderSearchResult):
    def __init__(self, hits: List[SearchHit]):
        """
        Initializes a SearchableKnowledgeProviderResult with a list of search hits.
        :param hits: A sorted list of SearchHit objects representing the search results.
        """
        self.hits = hits

    def _view(self) -> Dict[str, List[str]]:
        view: Dict[str, List[str]] = {}
        for hit in self.hits:
            if hit.document_name not in view:
                view[hit.document_name] = [hit.text_piece]
            else:
                if hit.text_piece not in view[hit.document_name]:
                    view[hit.document_name].append(hit.text_piece)
        return view

    def render(self) -> Optional[str]:
        if not self.hits:
            return None

        view = self._view()
        result = "I've found the following snippets in the knowledge base:\n"
        for document_name, snippet_list in view.items():
            result += f"Document Name: {document_name}\n"
            for snippet in snippet_list:
                result += f""""
SNIPPET BEGIN
{snippet}
SNIPPET END
"""
        return result

    def reduce_size(self) -> None:
        if self.hits:
            # the hits are sorted by distance, so we can just remove the last one
            self.hits.pop()

class SearchableKnowledgeProvider(KnowledgeProvider):
    def __init__(
        self,
        scope: str,
        model: Model = None,
        storage: Optional[Storage] = None,
        text_extractor: Optional[TextExtractor] = None,
        chunker: Chunker = NaiveChunker(),
        max_results: int = 10,
        max_distance: float = 0.2,
    ):
        """
        This class allows to store and search for text documents using vector search.

        :param scope: The name of the system instance.
        :type scope: str
        :param model: The model being utilized for processing operations.
        :type model: Model
        :param storage: Mechanism to store data.
        Defaults is run-time dependent.
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
        if model is None:
            model = Model("ollama/nomic-embed-text")

        if storage is None:
            storage = create_storage()

        if text_extractor is None:
            text_extractor = create_extractor()

        self.scope = scope
        self.model = model
        self.storage = storage
        self.text_extractor = text_extractor
        self.chunker = chunker
        self.max_results = max_results
        self.max_distance = max_distance

    async def add(self, document: Document):
        """
        Read a document using the unstructured library and add it to the knowledge base.

        :param document: The document to add.
        """

        if document.url is not None:
            content = await download_url(document.url)
        else:
            content = document.content

        whole_document = await self.text_extractor.extract_text(content, document.content_type)
        text_pieces = self.chunker.chunk(whole_document)

        # A single call is much faster than calling the model for each text piece
        embeddings = await self.model.embeddings(text_pieces)

        text_pieces = [TextPiece(text, embedding) for text, embedding in zip(text_pieces, embeddings)]
        await self.storage.store_text_piece(
            self.scope,
            document.id,
            document.name,
            text_pieces,
        )

    async def search(self, query: str) -> ProviderSearchResult:
        embeddings = await self.model.embeddings([query])
        hits = await self.storage.search_text(self.scope, embeddings[0], self.max_results, self.max_distance)
        debug(f"Search results for query '{query}': {len(hits)} hits found in knowledge '{self.scope}'")
        return SearchableKnowledgeProviderResult(hits)


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
