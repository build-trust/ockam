from typing_extensions import Protocol


class Chunker(Protocol):
    def chunk(self, text: str) -> list[str]:
        """
        Chunk the given text into smaller pieces.

        :param text: The text to be chunked.
        :return: A list of text chunks.
        """
        ...
