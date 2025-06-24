from typing import List, Dict


class TextPiece:
    text: str
    embedding: List[float]

    def __init__(self, text: str, embedding: List[float]):
        self.text = text
        self.embedding = embedding

    def __repr__(self) -> str:
        return f"TextPiece(text={self.text}, embedding={self.embedding[:3]}...)"


class SearchHit:
    document_id: str
    document_name: str
    text_piece: str
    distance: float

    def __init__(self, document_id: str, document_name: str, text_piece: str, distance: float = 0.0):
        self.document_id = document_id
        self.document_name = document_name
        self.text_piece = text_piece
        self.distance = distance

    def __hash__(self) -> int:
        return hash((self.document_id, self.text_piece))

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, SearchHit):
            return False
        return self.document_id == other.document_id and self.text_piece == other.text_piece

    def __repr__(self) -> str:
        return f"SearchHit(document_id={self.document_id}, text_piece={self.text_piece}, distance={self.distance})"

