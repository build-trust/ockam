from typing import Protocol, List, Optional
from ..search import TextPiece, SearchHit

class Storage(Protocol):
    async def store_document(self, namespace: str, id: str, name: str, text: str) -> None:
        """
        Store a whole document in the storage.

        :param namespace: The namespace to store the document in.
        :param id: The unique identifier for the document.
        :param text: The text content of the document.
        :raises Exception: If the document already exists.
        """
        ...

    async def store_text_piece(self, namespace: str, id: str, name: str, pieces: List[TextPiece]) -> None:
        """
        Store text pieces in the storage.

        :param namespace: The knowledge namespace to store the text pieces in.
        :param id: The unique identifier for the document.
        :param pieces: A list of TextPiece objects containing text and embedding.
        :raises Exception: If the document does not exist.
        """
        ...

    async def documents(self, namespace: str, id: Optional[str] = None) -> List[SearchHit]:
        """
        Retrieve whole documents from the storage.
        This method returns all documents if no document_id is specified.

        :param namespace: The knowledge namespace to search in.
        :param id: Optional document id to retrieve a specific document.
        :raises Exception: If the document is not found when document_id is specified.
        :return: List of SearchHit objects containing document id and their text content.
        """
        ...

    async def search_text(
        self, namespace: str, embedding: List[float], max_results: int, max_distance: float
    ) -> List[SearchHit]:
        """
        Search for text in the storage using vector similarity.
        This method returns a list of SearchHit objects that match the query.

        :param namespace: The knowledge namespace to search in.
        :param embedding: The query embedding vector.
        :param max_results: Maximum number of results to return.
        :param max_distance: Maximum distance threshold for matches, 0 means exact match.
        :return: List of SearchHit objects that match the criteria.
        """
        ...
