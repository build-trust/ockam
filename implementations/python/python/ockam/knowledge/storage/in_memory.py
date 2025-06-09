import numpy as np
from typing import Dict, List, Optional, Tuple
from scipy.spatial.distance import cosine

from ..search import SearchHit, TextPiece
from .protocol import Storage


class InMemory(Storage):
    text_pieces: Dict[str, Dict[str, Tuple[str, List[Tuple[str, np.ndarray]]]]]
    whole_documents: Dict[str, Tuple[str, str]]

    def __init__(self):
        self.text_pieces = {}
        self.whole_documents = {}

    async def store_document(self, namespace: str, id: str, name: str, text: str) -> None:
        self.whole_documents[id] = (name,text)

    async def documents(self, namespace: str, id: Optional[str] = None) -> List[SearchHit]:
        if id:
            if id in self.whole_documents:
                document = self.whole_documents[id]
                return [SearchHit(id, document[0], document[1])]
            else:
                raise Exception(f"Document {id} not found.")
        return [SearchHit(document_id, document[0], document[1]) for document_id, document in self.whole_documents.items()]

    async def store_text_piece(self, namespace: str, id: str, name: str, pieces: List[TextPiece]) -> None:
        if namespace not in self.text_pieces:
            self.text_pieces[namespace] = {}

        if id not in self.text_pieces[namespace]:
            self.text_pieces[namespace][id] = (name, [])

        processed_pieces: List[Tuple[str, np.ndarray]] = [(piece.text, np.array(piece.embedding, dtype=np.float32)) for piece in pieces]
        self.text_pieces[namespace][id][1].extend(processed_pieces)

    async def search_text(
        self, namespace: str, embedding: List[float], max_results: int, max_distance: float
    ) -> List[SearchHit]:
        if namespace not in self.text_pieces:
            return []

        embedding_array: np.ndarray = np.array(embedding, dtype=np.float32)
        hits: List[SearchHit] = []
        for document_id, document_pieces in self.text_pieces[namespace].items():
            name = document_pieces[0]
            pieces = document_pieces[1]
            for piece in pieces:
                # returned cosine distance is in the range [0, 2]
                distance: float = cosine(embedding_array, piece[1]) * 0.5
                if distance <= max_distance:
                    hits.append(SearchHit(document_id, name, piece[0], float(distance)))

        hits = sorted(hits, key=lambda hit: hit.distance)
        return hits[:max_results]
