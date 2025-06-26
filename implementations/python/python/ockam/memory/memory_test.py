import os
import pytest
import psycopg
from psycopg.rows import dict_row
from .memory import Memory


@pytest.fixture
def setup_env(monkeypatch):
    if "OCKAM_DATABASE_INSTANCE" not in os.environ:
        pytest.skip("Skipping test: OCKAM_DATABASE_INSTANCE not set in environment")

    monkeypatch.setenv("OCKAM_DATABASE_INSTANCE", os.environ.get("OCKAM_DATABASE_INSTANCE", "localhost:5432/test"))
    monkeypatch.setenv("OCKAM_DATABASE_USER", "postgres")
    monkeypatch.setenv("OCKAM_DATABASE_PASSWORD", "password")

    connection = psycopg.connect("postgresql://postgres@localhost:5432/test", row_factory=dict_row)

    with connection.cursor() as cur:  # pylint: disable=E1101
        cur.execute("""
                    CREATE TABLE IF NOT EXISTS conversation
                    (
                        tenant_id    TEXT,
                        scope        TEXT,
                        conversation TEXT,
                        message      TEXT
                    );
                    DELETE
                    FROM conversation;
                    """)
        connection.commit()  # pylint: disable=E1101


def test_add_and_get_message(setup_env):
    memory = Memory()
    scope = "scope"
    conversation = "conversation"
    message = {"text": "Hello world"}

    memory.add_message(scope, conversation, message)
    messages = memory.get_messages_only(scope, conversation)

    assert len(messages) == 1
    assert messages[0]["text"] == "Hello world"
    assert messages[0]["scope"] == scope
    assert messages[0]["conversation"] == conversation
