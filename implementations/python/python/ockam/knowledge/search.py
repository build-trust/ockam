from typing import List


class TextPiece:
    text: str
    embedding: List[float]

    def __init__(self, text: str, embedding: List[float]):
        self.text = text
        self.embedding = embedding

    def __repr__(self):
        return f"TextPiece(text={self.text}, embedding={self.embedding[:3]}...)"


class SearchHit:
    def __init__(self, document_name: str, text_piece: str, distance: float = 0.0):
        self.document_name = document_name
        self.text_piece = text_piece
        self.distance = distance

    def __hash__(self):
        return hash((self.document_name, self.text_piece))

    def __eq__(self, other):
        if not isinstance(other, SearchHit):
            return False
        return self.document_name == other.document_name and self.text_piece == other.text_piece

    def __repr__(self):
        return f"SearchHit(document_name={self.document_name}, text_piece={self.text_piece}, distance={self.distance})"


class SearchResults:
    def __init__(self):
        self.search_results = []

    def add(self, hits: List[SearchHit]):
        # sort hits by distance
        hits.sort(key=lambda x: x.distance, reverse=True)
        self.search_results.append(hits)

    def reduce_size(self):
        """
        Reduces the size of the search results.
        First by removing the oldest hits and then by removing the most distant hits.
        """
        if len(self.search_results) == 0:
            return

        # Remove the oldest hits
        if len(self.search_results) > 1:
            self.search_results.pop(0)
            return

        # Remove the most distant hit
        if len(self.search_results[0]) > 0:
            self.search_results[0].pop(0)

    def __repr__(self):
        return f"SearchResults(search_results={self.search_results})"

    def view(self):
        """
        Creates a deduplicated view of the search results, using document name as key.
        This view can be used to convert the search results into a string.
        """
        view = {}
        for hits in self.search_results:
            for hit in hits:
                if hit.document_name not in view:
                    view[hit.document_name] = [hit.text_piece]
                else:
                    if hit.text_piece not in view[hit.document_name]:
                        view[hit.document_name].append(hit.text_piece)
        return view


def test_search_result():
    hit1 = SearchHit("doc1", "text1", 0.2)
    hit2 = SearchHit("doc1", "text2", 0.1)
    hit3 = SearchHit("doc2", "text3", 0.6)
    hit4 = SearchHit("doc2", "text4", 0.5)

    search_result = SearchResults()
    search_result.add([hit1, hit2, hit3])

    view = search_result.view()
    assert len(view) == 2
    assert view[hit1.document_name] == [hit1.text_piece, hit2.text_piece]
    assert view[hit3.document_name] == [hit3.text_piece]

    search_result.add([hit1, hit2, hit3])
    view2 = search_result.view()
    assert view == view2

    search_result.reduce_size()
    view3 = search_result.view()
    assert view == view3

    search_result.add([hit4])
    view4 = search_result.view()
    assert len(view4) == 2
    assert view4[hit1.document_name] == [hit1.text_piece, hit2.text_piece]
    assert view4[hit3.document_name] == [hit3.text_piece, hit4.text_piece]

    search_result.reduce_size()
    view5 = search_result.view()
    assert len(view5) == 1
    assert view5[hit4.document_name] == [hit4.text_piece]

    search_result.reduce_size()
    view6 = search_result.view()
    assert len(view6) == 0

    search_result.reduce_size()
