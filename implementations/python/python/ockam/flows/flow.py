import secrets
import re

from .operation import FlowOperation, END
from .flow_state import FlowState, FlowEdge, vertex_to_id
from ..agents import validate_name
from ..nodes import LocalNodeProtocol
from ..nodes.message import (
    ConversationSnippet,
    Error,
    GetIdentifierRequest,
    GetIdentifierResponse,
    MessageConverter,
    FlowReference,
    Reference,
    AssistantMessage,
    UserMessage,
    StreamedConversationSnippet,
)

from ..ockam_in_rust_for_python import info, debug


class Flow:
    def __init__(self, name=None, iteration_timeout=120, iteration_limit=20):
        if name is None:
            name = secrets.token_hex(12)
        else:
            validate_name(name)

        self.name = name
        self.iteration_timeout = iteration_timeout
        self.iteration_limit = iteration_limit
        self.state = FlowState(name)

    def add(
        self,
        from_: Reference | str,
        to: Reference | str,
        condition: str = "",
        operation: FlowOperation = FlowOperation.ROUTE,
    ):
        vertex = from_
        edge = FlowEdge(to, condition, operation)
        self.state.add(vertex, edge)

    @staticmethod
    async def start(node: LocalNodeProtocol, flow):
        name = flow.name
        worker = FlowWorker(
            name, node, flow, iteration_limit=flow.iteration_limit, iteration_timeout=flow.iteration_timeout
        )
        await node.start_worker(name, worker)
        return FlowReference(name, node)


class FlowWorker:
    def __init__(self, name: str, node: LocalNodeProtocol, flow: Flow, iteration_limit: int, iteration_timeout: int):
        self.name = name
        self.node = node
        self.flow = flow
        self.converter = MessageConverter.create(node)
        self.iteration_limit = iteration_limit
        self.iteration_timeout = iteration_timeout

    async def handle_message(self, context, message):
        try:
            message = self.converter.message_from_json(message)
            handlers = {
                ConversationSnippet: self.handle__conversation_snippet,
                StreamedConversationSnippet: self.handle__conversation_snippet,
                GetIdentifierRequest: self.handle__get_identifier_request,
            }

            handler = handlers.get(type(message))
            if handler is not None:
                reply = await handler(message)
            else:
                reply = Error(f"Unexpected Message: {message}")

            if reply is not None:
                context.reply(self.converter.message_to_json(reply))
        except Exception as e:
            error = Error(str(e))
            context.reply(self.converter.message_to_json(error))

    async def handle__get_identifier_request(self, message: GetIdentifierRequest) -> GetIdentifierResponse:
        name_snake_case = self.name.lower().replace(" ", "_")
        node_identifier = await self.node.identifier()
        agent_identifier = f"{node_identifier}/{name_snake_case}"
        return GetIdentifierResponse(message.scope, message.conversation, agent_identifier)

    async def handle__conversation_snippet(self, snippet: ConversationSnippet) -> ConversationSnippet:
        stream = type(snippet) is StreamedConversationSnippet
        if stream:
            snippet = snippet.snippet

        if not snippet.scope:
            snippet.scope = secrets.token_hex(16)

        if not snippet.conversation:
            snippet.conversation = secrets.token_hex(16)

        debug(f"INIT: {snippet}\n\n")

        self.flow.state.reset()

        for i in range(self.iteration_limit):
            edge = self.flow.state.next_edge(snippet)

            if not edge:
                raise RuntimeError("Flow can't converge")

            next_vertex = edge.destination
            snippet_to_send = snippet

            if edge.operation == FlowOperation.TRY_AGAIN:
                # Remove the last response and retry
                snippet.messages.pop()

            if edge.operation == FlowOperation.EVALUATE:
                # Send only the last message
                conversation = secrets.token_hex(16)
                last_message = snippet.messages[-1]
                # convert AssistantMessage to UserMessage to not confuse the next model
                if isinstance(last_message, AssistantMessage):
                    last_message = UserMessage(content=last_message.content)
                snippet_to_send = ConversationSnippet(snippet.scope, conversation, [last_message])

            if next_vertex == END:
                info(f"Flow converged in {i} iterations")
                snippet.messages[:] = snippet.messages[-1:]
                return snippet

            if not isinstance(next_vertex, Reference):
                raise RuntimeError(f"Unexpected next_vertex: {next_vertex}")

            next_vertex_id = vertex_to_id(next_vertex)
            debug(f"Iteration: {i} To: {next_vertex_id} Sending: {snippet_to_send}\n\n")

            reply = await next_vertex.send_and_receive_request(
                snippet_to_send, converter=self.converter, timeout=self.iteration_timeout
            )

            reply_to_append = reply
            if edge.operation == FlowOperation.EVALUATE:
                reply_to_append = ConversationSnippet(reply.scope, reply.conversation, [])
                for message in reply.messages:
                    reply_to_append.messages.append(UserMessage(content=message.content))

            snippet.messages.extend(reply_to_append.messages)
            debug(f"Iteration: {i} From {next_vertex_id} Received: {reply}\n\n")

        raise RuntimeError(f"Flow did not converge in {self.iteration_limit} iterations")
