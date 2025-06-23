from typing import Optional, List


from .extractors import TextExtractor, create_extractor
from .protocol import KnowledgeProvider, Storage
from .in_memory import InMemory
from .search import SearchHit, SearchResults
from .util import download_url


class UnsearchableKnowledge(KnowledgeProvider):
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
        max_knowledge_size: int = 4096,
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
        self.search_results = SearchResults()
        self.max_knowledge_size = max_knowledge_size

    async def search(self) -> List[SearchHit]:
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

    async def search_knowledge(self, _scope: Optional[str], conversation: Optional[str], _query: str) -> Optional[str]:
        hits = await self.search()
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
