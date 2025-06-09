from collections import defaultdict
import threading


class ConversationHistory:
    def __init__(self):
        self.instructions = []
        self.conversations = defaultdict(lambda: defaultdict(list[dict]))
        self.lock = threading.RLock()

    def set_instructions(self, instructions: dict):
        with self.lock:
            self.instructions = [instructions]

    def add_message(self, scope: str, conversation: str, message: dict):
        with self.lock:
            self.conversations[scope][conversation].append(message)

    def get_messages(self, scope: str, conversation: str) -> list[dict]:
        with self.lock:
            return self.instructions + self.get_messages_only(scope, conversation)

    def get_messages_only(self, some_scope: str, some_conversation: str) -> list[dict]:
        with self.lock:
            if some_conversation is None:
                if some_scope is None:
                    messages = [
                        message
                        for scope in self.conversations.values()
                        for conversation in scope.values()
                        for message in conversation
                    ]
                else:
                    messages = [
                        message
                        for conversation in self.conversations.get(some_scope, {}).values()
                        for message in conversation
                    ]
            else:
                messages = self.conversations.get(some_scope, {}).get(some_conversation, [])

            # add the scope and conversation fields if they are defined
            result = []
            for message in messages:
                if some_scope is not None:
                    message["scope"] = some_scope
                if some_conversation is not None:
                    message["conversation"] = some_conversation
                result.append(message)
            return result

    def display_conversations(self):
        with self.lock:
            for scope, conversations in self.conversations.items():
                print(f"Scope: {scope}")
                for conversation, messages in conversations.items():
                    print(f"  Conversation: {conversation}")
                    for i, message in enumerate(messages):
                        print(f"    Message {i + 1}: {message}")
