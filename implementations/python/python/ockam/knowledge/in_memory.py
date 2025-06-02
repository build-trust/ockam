import numpy as np
from typing import List
from scipy.spatial.distance import cosine

from .search import SearchHit, TextPiece
from .protocol import Storage


class InMemory(Storage):
    def __init__(self):
        self.text_pieces = {}
        self.whole_documents = {}

    async def initialize(self):
        pass

    async def store_document(self, knowledge: str, document_name: str, text: str):
        """Store a whole document in the storage"""
        self.whole_documents[document_name] = text

    async def documents(self, knowledge: str, document_name: str = None) -> list[SearchHit]:
        """
        Returns documents from the storage.
        """
        if document_name:
            if document_name in self.whole_documents:
                return [SearchHit(document_name, self.whole_documents[document_name])]
            else:
                raise RuntimeError(f"Document {document_name} not found.")
        return [SearchHit(document_name, text) for document_name, text in self.whole_documents.items()]

    async def store_text_piece(self, knowledge: str, document_name: str, pieces: List[TextPiece]):
        if knowledge not in self.text_pieces:
            self.text_pieces[knowledge] = {}

        if document_name not in self.text_pieces[knowledge]:
            self.text_pieces[knowledge][document_name] = []

        pieces = [(piece.text, np.array(piece.embedding, dtype=np.float32)) for piece in pieces]
        self.text_pieces[knowledge][document_name].extend(pieces)

    async def search_text(
        self, knowledge: str, embedding: List[float], max_results: int, max_distance: float
    ) -> List[SearchHit]:
        if knowledge not in self.text_pieces:
            return []

        embedding = np.array(embedding, dtype=np.float32)
        hits = []
        for document_name, pieces in self.text_pieces[knowledge].items():
            for piece in pieces:
                # returned cosine distance is in the range [0, 2]
                distance = cosine(embedding, piece[1]) * 0.5
                if distance <= max_distance:
                    hits.append(SearchHit(document_name, piece[0], float(distance)))

        hits = sorted(hits, key=lambda hit: hit.distance)
        return hits[:max_results]
