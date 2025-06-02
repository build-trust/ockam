from .protocol import Chunker


class NaiveChunker(Chunker):
    def __init__(self, max_characters: int = 256, overlap: int = 16):
        """
        Initialize the NaiveChunker with an optional overlap.

        :param max_characters: Maximum number of characters in each chunk.
        :param overlap: Number of characters to overlap between chunks.
        """
        self.overlap = overlap
        self.max_characters = max_characters

    def chunk(self, text: str) -> list[str]:
        pieces = []
        start = 0

        while start < len(text):
            end = start + min(self.max_characters, len(text))

            piece = text[start:end]
            pieces.append(piece)

            start = end - self.overlap

        return pieces


def test_naive():
    chunker = NaiveChunker(max_characters=50, overlap=10)
    text = "This is a test text to be chunked into smaller pieces. It should work well with the naive chunker."
    chunks = chunker.chunk(text)

    assert len(chunks) == 3
    assert chunks[0] == "This is a test text to be chunked into smaller pie"
    assert chunks[1] == "maller pieces. It should work well with the naive "
    assert chunks[2] == "the naive chunker."
