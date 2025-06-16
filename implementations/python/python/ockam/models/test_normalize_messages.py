from ockam.models.model import normalize_messages
from ockam.nodes.message import UserMessage, AssistantMessage, SystemMessage


def test_conversion_from_conversation_message():
    messages = [
        SystemMessage("you are a helpful assistant"),
        UserMessage("hello"),
        AssistantMessage("hello user!"),
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert all(isinstance(m, dict) for m in result)
    assert len(result) == 3
    assert result[0]["role"] == "system"
    assert result[0]["content"] == "you are a helpful assistant"
    assert result[1]["role"] == "user"
    assert result[1]["content"] == "hello"
    assert result[2]["role"] == "assistant"
    assert result[2]["content"] == "hello user!"


def test_removal_of_empty_content():
    messages = [{"role": "user", "content": ""}, {"role": "assistant", "content": "hi"}]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert all("content" in m for m in result)
    assert len(result) == 1
    assert result[0]["role"] == "assistant"


def test_removal_of_empty_tool_calls_tools_supported():
    messages = [{"role": "assistant", "content": "hi", "tool_calls": []}]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert "tool_calls" not in result[0]


def test_tool_role_conversion_and_tool_calls_removal():
    messages = [
        {"role": "tool", "content": "", "tool_calls": [1]},
        {"role": "tool", "content": "", "tool_calls": []},
        {"role": "tool", "content": "output", "name": "toolname"},
    ]
    # Only the last message has a name, so only it should remain, and its role should be 'assistant'
    result = normalize_messages(messages, tools_supported=False, forced_assistant_answer_supported=True)
    assert len(result) == 1
    assert result[0]["role"] == "assistant"
    assert "tool_calls" not in result[0]


def test_removal_of_scope_and_conversation():
    messages = [
        {"role": "user", "content": "hi", "scope": "abc", "conversation": "xyz"},
        {"role": "assistant", "content": "ok"},
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert all("scope" not in m and "conversation" not in m for m in result)


def test_filtering_of_useless_messages():
    messages = [
        {"role": "system"},  # no content
        {"role": "user"},  # no content
        {"role": "assistant"},  # no content/tool_calls
        {"role": "assistant", "tool_calls": []},  # no content/tool_calls
        {"role": "tool"},  # no name
        {"role": "tool", "name": "toolname"},  # valid
        {"role": "user", "content": "hi"},  # valid
        {"role": "assistant", "content": "ok"},  # valid
        {"role": "assistant", "tool_calls": [1]},  # valid
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    roles = [m["role"] for m in result]
    assert roles == ["tool", "user", "assistant", "assistant"]


def test_message_with_content_and_tool_calls():
    messages = [
        {"role": "assistant", "content": "foo", "tool_calls": [1]},
        {"role": "assistant", "tool_calls": [2]},
        {"role": "assistant", "content": "foo"},
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert len(result) == 3
    assert all(m["role"] == "assistant" for m in result)


def test_empty_input():
    assert normalize_messages([], tools_supported=True, forced_assistant_answer_supported=True) == []
    assert normalize_messages([], tools_supported=False, forced_assistant_answer_supported=False) == []


def test_tool_calls_preserved_when_not_empty():
    messages = [{"role": "assistant", "content": "hi", "tool_calls": [1, 2]}]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert "tool_calls" in result[0]
    assert result[0]["tool_calls"] == [1, 2]


def test_tool_calls_removed_when_tools_not_supported():
    messages = [{"role": "assistant", "content": "hi", "tool_calls": [1, 2]}]
    result = normalize_messages(messages, tools_supported=False, forced_assistant_answer_supported=True)
    assert "tool_calls" not in result[0]


def test_scope_and_conversation_only_removed():
    messages = [
        {"role": "user", "scope": "abc"},
        {"role": "assistant", "conversation": "xyz"},
        {"role": "user", "content": "hi", "scope": "abc", "conversation": "xyz"},
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert all("scope" not in m and "conversation" not in m for m in result)
    assert len(result) == 1
    assert result[0]["content"] == "hi"


def test_system_message_with_content_kept():
    messages = [
        {"role": "system", "content": "instructions"},
        {"role": "system"},
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert len(result) == 1
    assert result[0]["role"] == "system"
    assert result[0]["content"] == "instructions"


def test_tool_message_with_name_kept():
    messages = [
        {"role": "tool", "name": "toolname"},
        {"role": "tool"},
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert len(result) == 1
    assert result[0]["role"] == "tool"
    assert result[0]["name"] == "toolname"


def test_compacts_consecutive_messages_with_same_role_and_fields():
    messages = [
        {"role": "user", "content": "hello"},
        {"role": "user", "content": "world"},
        {"role": "assistant", "content": "hi"},
        {"role": "assistant", "content": "there"},
        {"role": "user", "content": "again", "foo": 1},
        {"role": "user", "content": "bar", "foo": 1},
        {"role": "user", "content": "baz", "foo": 2},
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert len(result) == 4
    assert result[0]["content"] == "helloworld"
    assert result[1]["content"] == "hithere"
    assert result[2]["content"] == "againbar"
    assert result[3]["content"] == "baz"


def test_last_message_assistant_without_forced_answer():
    messages = [
        {"role": "assistant", "content": "Create me a new recipe for a cake."},
        {"role": "assistant", "content": "First we need to gather ingredients, such as"},
    ]
    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=True)
    assert len(result) == 1
    assert result[0]["role"] == "assistant"

    result = normalize_messages(messages, tools_supported=True, forced_assistant_answer_supported=False)
    assert len(result) == 1
    assert result[0]["role"] == "user"
