from typing import Protocol, List, Optional
from .search import SearchHit, TextPiece


class KnowledgeProvider(Protocol):
    async def search_knowledge(
        self, scope: Optional[str], conversation: Optional[str], query: str
    ) -> Optional[str]: ...


class Storage(Protocol):
    async def store_document(self, knowledge: str, document_name: str, text: str):
        """Store a whole document in the storage"""
        ...

    async def documents(self, knowledge: str, document_name: str = None) -> List[SearchHit]:
        """Returns documents from the storage"""
        ...

    async def store_text_piece(self, knowledge: str, document_name: str, pieces: List[TextPiece]):
        """Store text in the storage"""
        ...

    async def search_text(
        self, knowledge: str, embedding: List[float], max_results: int, max_distance: float
    ) -> List[SearchHit]:
        """Search for text in the storage"""
        ...
