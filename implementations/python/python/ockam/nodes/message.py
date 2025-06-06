import cattr
import json
import bisect
import secrets

from dataclasses import dataclass, field
from enum import Enum
from typing import Union, Any

from .local import LocalNodeProtocol
from .remote import RemoteNode

from ..ockam_in_rust_for_python import Mailbox


class ConversationRole(Enum):
    USER = "user"
    SYSTEM = "system"
    ASSISTANT = "assistant"
    TOOL = "tool"


@dataclass
class FunctionToolCall:
    name: str
    arguments: str = field(default_factory=str)


@dataclass
class ToolCall:
    id: str
    function: FunctionToolCall
    type: str = "function"


class Phase(Enum):
    THINKING = "thinking"
    PLANNING = "planning"
    EXECUTING = "executing"


@dataclass
class UserMessage:
    content: str = ""
    phase: Phase = Phase.EXECUTING
    role: ConversationRole = ConversationRole.USER


@dataclass
class SystemMessage:
    content: str
    role: ConversationRole = ConversationRole.SYSTEM


@dataclass
class AssistantMessage:
    content: str = ""
    phase: Phase = Phase.EXECUTING
    role: ConversationRole = ConversationRole.ASSISTANT
    tool_calls: list[ToolCall] = field(default_factory=list)


@dataclass
class ToolCallResponseMessage:
    content: str
    tool_call_id: str
    name: str
    phase: Phase = Phase.EXECUTING
    role: ConversationRole = ConversationRole.TOOL


ConversationMessage = Union[UserMessage, SystemMessage, AssistantMessage, ToolCallResponseMessage]


class ReferenceType(Enum):
    AGENT = "agent"
    FLOW = "flow"
    SQUAD = "squad"


@dataclass
class ReceiveReference:
    mailbox: Mailbox
    converter: Any

    async def receive_message(self, timeout=None):
        response_json = await self.mailbox.receive(timeout=timeout)

        response = self.converter.message_from_json(response_json)
        if isinstance(response, Error):
            raise Exception(response.message)

        return response.messages


# TODO: There is some overlap in functionality between Reference class and Local/Remote node classes
class Reference:
    # TODO: Policies
    name: str
    node: LocalNodeProtocol
    reference_type: ReferenceType

    def __init__(self, name: str, node: LocalNodeProtocol | RemoteNode, reference_type: ReferenceType):
        self.name = name
        self.reference_type = reference_type

        if isinstance(node, RemoteNode):
            self.node = node.node_that_this_object_is_on
            self.name_of_remote_node = node.name_of_remote_node
        else:
            self.node = node
            self.name_of_remote_node = None

    async def identifier(self, scope="", conversation=""):
        response = await self.send_and_receive_request(GetIdentifierRequest(scope, conversation))
        return response.identifier

    async def send(self, message, scope="", conversation="", timeout=None):
        request = ConversationSnippet(scope, conversation, [UserMessage(message)])
        response = await self.send_and_receive_request(request, timeout=timeout)
        return response.messages

    async def send_stream(self, message, scope="", conversation="", timeout=None):
        request = StreamedConversationSnippet(ConversationSnippet(scope, conversation, [UserMessage(message)]))
        async for response in self.send_request_stream(request, timeout=timeout):
            yield response

    async def send_message(self, message, scope="", conversation=""):
        request = ConversationSnippet(scope, conversation, [UserMessage(message)])
        return await self.send_request(request)

    async def send_request(self, message, converter=None) -> ReceiveReference:
        if converter is None:
            converter = MessageConverter.create(self.node)

        message_json = converter.message_to_json(message)

        import secrets

        address = secrets.token_hex(12)
        mailbox = await self.node.create_mailbox(address)

        if self.name_of_remote_node:
            await mailbox.send_to_remote(self.name_of_remote_node, self.name, message_json)
        else:
            await mailbox.send(self.name, message_json)

        return ReceiveReference(mailbox, converter)

    async def send_and_receive_request(self, message, converter=None, timeout=None):
        if converter is None:
            converter = MessageConverter.create(self.node)

        message_json = converter.message_to_json(message)

        if self.name_of_remote_node:
            response_json = await self.node.send_and_receive_to_remote(
                self.name_of_remote_node, self.name, message_json, timeout=timeout
            )
        else:
            response_json = await self.node.send_and_receive(self.name, message_json, timeout=timeout)

        response = converter.message_from_json(response_json)
        if isinstance(response, Error):
            raise Exception(response.message)
        return response

    async def send_request_stream(self, message, converter=None, timeout=None):
        if converter is None:
            converter = MessageConverter(self.node)
        message_json = converter.message_to_json(message)

        random_address = f"message_{secrets.token_hex(16)}"
        mailbox = await self.node.create_mailbox(random_address)

        if self.name_of_remote_node:
            await mailbox.send_to_remote(self.name_of_remote_node, self.name, message_json)
        else:
            await mailbox.send(self.name, message_json)

        expected_part_nb = 1
        out_of_order_messages = []
        finished = False
        while not finished:
            response_json = await mailbox.receive(timeout=timeout)
            response = converter.message_from_json(response_json)
            if isinstance(response, Error):
                raise Exception(response.message)

            # even if we make a streaming request, the response might not be streaming if the downstream
            # agent does not support streaming
            if type(response) is StreamedConversationSnippet:
                if response.part_nb > expected_part_nb:
                    bisect.insort(out_of_order_messages, response)
                else:
                    expected_part_nb = response.part_nb + 1
                    finished = response.finished
                    yield response

                if len(out_of_order_messages) > 0:
                    while len(out_of_order_messages) > 0:
                        m = out_of_order_messages[0]
                        if m.part_nb == expected_part_nb:
                            del out_of_order_messages[0]
                            expected_part_nb = m.part_nb + 1
                            finished = m.finished
                            yield m
                        else:
                            break
            else:
                finished = True
                yield response

    @property
    def node_name(self):
        return self.node.name


class AgentReference(Reference):
    def __init__(
        self,
        name: str,
        node: LocalNodeProtocol | RemoteNode,
    ):
        super().__init__(name, node, ReferenceType.AGENT)


@dataclass
class FlowReference(Reference):
    def __init__(
        self,
        name: str,
        node: LocalNodeProtocol | RemoteNode,
    ):
        super().__init__(name, node, ReferenceType.FLOW)


@dataclass
class SquadReference(Reference):
    def __init__(
        self,
        name: str,
        node: LocalNodeProtocol | RemoteNode,
    ):
        super().__init__(name, node, ReferenceType.SQUAD)


class MessageType(Enum):
    CONVERSATION_SNIPPET = "conversation_snippet"
    STREAMED_CONVERSATION_SNIPPET = "streamed_conversation_snippet"
    GET_IDENTIFIER_REQUEST = "get_identifier_request"
    GET_IDENTIFIER_RESPONSE = "get_identifier_response"
    GET_CONVERSATIONS_REQUEST = "get_conversations_request"
    GET_CONVERSATIONS_RESPONSE = "get_conversations_response"
    ERROR = "error"


@dataclass
class GetIdentifierRequest:
    scope: str
    conversation: str
    type: MessageType = MessageType.GET_IDENTIFIER_REQUEST


@dataclass
class GetIdentifierResponse:
    scope: str
    conversation: str
    identifier: str
    type: MessageType = MessageType.GET_IDENTIFIER_RESPONSE


@dataclass
class GetConversationsRequest:
    scope: None | str
    conversation: None | str
    type: MessageType = MessageType.GET_CONVERSATIONS_REQUEST


@dataclass
class GetConversationsResponse:
    conversations: list[dict]
    type: MessageType = MessageType.GET_CONVERSATIONS_RESPONSE


@dataclass
class Error:
    message: str
    type: MessageType = MessageType.ERROR


@dataclass
class ConversationSnippet:
    scope: str = ""
    conversation: str = ""
    messages: list[ConversationMessage] = field(default_factory=list)
    type: MessageType = MessageType.CONVERSATION_SNIPPET

    def compact_assistant_messages(self):
        messages = self.messages
        all_messages = []
        assistant_message = None
        for m in messages:
            if isinstance(m, AssistantMessage):
                if assistant_message is None:
                    assistant_message = m
                else:
                    assistant_message.content += m.content
            else:
                all_messages.append(m)

        if assistant_message is not None:
            all_messages.append(assistant_message)
        self.messages = all_messages
        return self


@dataclass
class StreamedConversationSnippet:
    snippet: ConversationSnippet
    part_nb: int = 0
    finished: bool = False
    type: MessageType = MessageType.STREAMED_CONVERSATION_SNIPPET

    # This is required to sort the received snippets when they are received out of order.
    def __lt__(self, other):
        return self.part_nb < other.part_nb


Message = Union[
    ConversationSnippet,
    StreamedConversationSnippet,
    GetIdentifierRequest,
    GetIdentifierResponse,
    GetConversationsRequest,
    GetConversationsResponse,
    Error,
]

CACHE = None


class MessageConverter:
    @staticmethod
    def create(node: LocalNodeProtocol):
        global CACHE
        if CACHE:
            return CACHE

        CACHE = MessageConverter(node)
        return CACHE

    def __init__(self, node):
        self.node = node
        self.converter = cattr.Converter()
        self._register_hooks()

    def _register_hooks(self):
        # Reference
        def unstructure_reference(obj: Reference) -> dict:
            return {
                "name": obj.name,
                "type": obj.type.value,
                "node": obj.node.name,
            }

        self.converter.register_unstructure_hook(AgentReference, unstructure_reference)
        self.converter.register_unstructure_hook(FlowReference, unstructure_reference)
        self.converter.register_unstructure_hook(SquadReference, unstructure_reference)

        @self.converter.register_structure_hook
        def structure_reference(obj: dict, cls) -> Reference:
            typ = obj.get("type")
            if typ is None:
                raise ValueError("Missing 'type' field in Reference")
            mapping = {
                "agent": AgentReference,
                "flow": FlowReference,
                "squad": SquadReference,
            }
            reference_cls = mapping.get(typ)
            if reference_cls is None:
                raise ValueError(f"Unknown reference type: {typ}")

            node_name = obj.get("node")
            if node_name == self.node.name:
                node = self.node
            else:
                node = RemoteNode(self.node, node_name)

            data = dict(obj)
            data["node"] = node

            return reference_cls(name=data["name"], node=data["node"])

        # ConversationRole
        @self.converter.register_unstructure_hook
        def unstructure_conversation_role(enum_obj: ConversationRole) -> str:
            return enum_obj.value

        @self.converter.register_structure_hook
        def structure_conversation_role(data: str, cls) -> ConversationRole:
            return cls(data)

        # ConversationMessage
        @self.converter.register_structure_hook
        def structure_conversation_message(obj: dict, cls) -> ConversationMessage:
            role = obj.get("role")
            mapping = {
                "user": UserMessage,
                "system": SystemMessage,
                "assistant": AssistantMessage,
                "tool": ToolCallResponseMessage,
            }
            typ = mapping.get(role)
            if typ is None:
                raise ValueError(f"Unknown conversation role: {role}")
            return self.converter.structure(obj, typ)

        # MessageType
        @self.converter.register_unstructure_hook
        def unstructure_message_type(enum_obj: MessageType) -> str:
            return enum_obj.value

        @self.converter.register_structure_hook
        def structure_message_type(data: str, cls) -> MessageType:
            return cls(data)

        # Message
        @self.converter.register_structure_hook
        def structure_message(obj: dict, cls) -> Message:
            typ = obj.get("type")
            if typ is None:
                raise ValueError("Missing 'type' field in Message")
            mapping = {
                "conversation_snippet": ConversationSnippet,
                "streamed_conversation_snippet": StreamedConversationSnippet,
                "get_identifier_request": GetIdentifierRequest,
                "get_identifier_response": GetIdentifierResponse,
                "get_conversations_request": GetConversationsRequest,
                "get_conversations_response": GetConversationsResponse,
                "error": Error,
            }
            message_cls = mapping.get(typ)
            if message_cls is None:
                raise ValueError(f"Unknown message type: {typ}")

            return self.converter.structure(obj, message_cls)

    def _unstructure_nested_map(self, nested_map: dict) -> dict:
        unstructured = {}
        for k, v in nested_map.items():
            if isinstance(v, Reference):
                unstructured[k] = self.converter.unstructure(v)
            else:
                unstructured[k] = v
        return unstructured

    def _structure_nested_map(self, nested_map: dict) -> dict[str, Reference]:
        structured = {}
        for k, v in nested_map.items():
            if isinstance(v, dict) and "type" in v:
                structured[k] = self.converter.structure(v, Reference)
            else:
                structured[k] = v
        return structured

    def conversation_message_from_dict(self, data: dict) -> ConversationMessage:
        return self.converter.structure(data, ConversationMessage)

    def message_from_dict(self, data: dict) -> Message:
        return self.converter.structure(data, Message)

    def message_to_dict(self, message: Message | ConversationMessage) -> dict:
        return self.converter.unstructure(message)

    def message_from_json(self, data: str) -> Message:
        return self.message_from_dict(json.loads(data))

    def message_to_json(self, message: Message) -> str:
        return json.dumps(self.message_to_dict(message))
