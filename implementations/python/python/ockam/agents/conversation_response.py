from ockam.nodes.message import ConversationMessage, StreamedConversationSnippet, ConversationSnippet, Phase


class ConversationResponse:
    def __init__(self, scope: str, conversation: str, stream: bool = False):
        self.scope = scope
        self.conversation = conversation
        self.stream = stream
        self.counter = 0

    def make_snippet(self, message: ConversationMessage, finished: bool = False):
        if self.stream:
            self.counter += 1
            return StreamedConversationSnippet(
                ConversationSnippet(self.scope, self.conversation, [message]), self.counter, finished
            )
        else:
            return ConversationSnippet(self.scope, self.conversation, [message])

    def make_finished_snippet(self):
        self.counter += 1
        return StreamedConversationSnippet(ConversationSnippet(self.scope, self.conversation, []), self.counter, True)
