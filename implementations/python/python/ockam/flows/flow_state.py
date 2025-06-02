import string
import re

from dataclasses import dataclass, field
from typing import Dict, Union

from ..nodes.message import Reference, ConversationSnippet
from .operation import FlowOperation, START

from ..ockam_in_rust_for_python import debug

FlowVertex = Union[
    str,
    Reference,
]


class FlowEdge:
    def __init__(self, destination: FlowVertex, condition: str, operation: FlowOperation):
        self.destination = destination
        self.condition = condition
        self.operation = operation


def vertex_to_id(vertex: FlowVertex) -> str:
    if isinstance(vertex, Reference):
        return f"{vertex.node.name}__{vertex.type.value}__{vertex.name}"

    return vertex


def route(edges: Dict[str, FlowEdge], conversation_snippet: ConversationSnippet) -> FlowEdge | None:
    last_output = conversation_snippet.messages[-1].content

    edge = None
    if last_output:
        # Truncate output for some degree of robustness
        last_output = last_output.splitlines()[0]
        last_output = last_output.lower()
        last_output = last_output.translate(str.maketrans("", "", string.punctuation))
        last_output = re.sub(r"\s+", " ", last_output).strip()
        if last_output:
            edge = edges.get(last_output)

    if not edge:
        edge = edges.get("")
    else:
        # Remove the condition message
        conversation_snippet.messages.pop()

    if not edge:
        return None

    debug(f"Route: {last_output} -> {edge}")

    return edge


@dataclass
class FlowState:
    name: str
    graph: Dict[str, Dict[str, FlowEdge]] = field(default_factory=dict)
    current_vertex: FlowVertex = field(default=START)

    def reset(self):
        self.current_vertex = START

    def add(self, vertex: FlowVertex, edge: FlowEdge):
        vertex_id = vertex_to_id(vertex)
        existing_edge = self.graph.get(vertex_id)

        # TODO: Raise exception on override
        if existing_edge:
            existing_edge[edge.condition] = edge
        else:
            self.graph[vertex_id] = {edge.condition: edge}

        debug(f"Added to Flow: {vertex_id} -> {edge}")

    def next_edge(self, conversation_snippet: ConversationSnippet) -> FlowEdge:
        current_vertex_id = vertex_to_id(self.current_vertex)
        edges = self.graph.get(current_vertex_id)

        if not edges:
            raise KeyError(f"Current vertex id '{current_vertex_id}' not found in graph")

        edge = route(edges, conversation_snippet)

        if edge:
            self.current_vertex = edge.destination

        return edge
