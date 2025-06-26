import json
import psycopg
import threading

from os import environ
from psycopg.rows import dict_row
from collections import defaultdict

from urllib.parse import quote


class Memory:
    _logger = None

    @classmethod
    def class_logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ockam import get_logger

            cls._logger = get_logger("memory")
            return cls._logger

    def __init__(self):
        self.logger = Memory.class_logger()
        self.tenant_id = None
        self.db_url = None
        self.connection = None
        self.lock = threading.RLock()

        self.instructions = []
        self.conversations = defaultdict(lambda: defaultdict(list[dict]))

        self.initialize_database()
        self.load_conversations()

    def initialize_database(self):
        db_instance = environ.get("OCKAM_DATABASE_INSTANCE")
        self.tenant_id = environ.get("OCKAM_DATABASE_USER")
        password = environ.get("OCKAM_DATABASE_PASSWORD")

        self.db_url = f"postgresql://{self.tenant_id}:{quote(password)}@{db_instance}"
        db_url_anonymized = f"postgresql://*****:*****@{db_instance}"
        self.logger.info("Connecting to database at %s", db_url_anonymized)

        self.connection: psycopg.Connection[dict] = psycopg.connect(self.db_url, row_factory=dict_row)

    def load_conversations(self):
        with self.connection.cursor() as cur:  # pylint: disable=E1101
            cur.execute("SELECT scope, conversation, message FROM conversation")
            rows = cur.fetchall()
            for row in rows:
                message = json.loads(row["message"])
                self.conversations[row["scope"]][row["conversation"]].append(message)

    def set_instructions(self, instructions: dict):
        with self.lock:
            self.instructions = [instructions]

    def add_message(self, scope: str, conversation: str, message: dict):
        with self.lock:
            self.conversations[scope][conversation].append(message)
            with self.connection.cursor() as cur:  # pylint: disable=E1101
                cur.execute(
                    "INSERT INTO conversation (tenant_id, scope, conversation, message) VALUES (%s, %s, %s, %s)",
                    (self.tenant_id, scope, conversation, json.dumps(message)),
                )
                self.connection.commit()  # pylint: disable=E1101

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
