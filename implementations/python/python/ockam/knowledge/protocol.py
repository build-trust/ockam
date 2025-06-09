from typing import Protocol, List, Optional
from .search import SearchHit
import secrets


class Document:
    name: str
    id: str
    content: Optional[str]
    url: Optional[str]
    content_type: Optional[str]

    def __init__(self, name: str, content: Optional[str] = None, url: Optional[str] = None, content_type: Optional[str] = None,  id: Optional[str] = None):
        if content is None and url is None:
            raise ValueError("Either content or url must be provided")

        if id is None:
            id = secrets.token_hex(16)
        self.name = name
        self.content = content
        self.url = url
        self.content_type = content_type
        self.id = id

    @staticmethod
    def inline(name: str, content: str, content_type: Optional[str], id: Optional[str] = None):
        return Document(name, content=content, content_type=content_type, id=id)

    @staticmethod
    def url(name: str, url: str, content_type: Optional[str], id: Optional[str] = None):
        return Document(name, url=url, content_type=content_type, id=id)



class KnowledgeProvider(Protocol):
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

