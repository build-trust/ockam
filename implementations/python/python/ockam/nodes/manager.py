import dill
import base64

from .local import LocalNode
from ..nodes.request import StartAgentResponse, StartAgentRequest
from ..nodes.request import StartAgentsResponse, StartAgentsRequest
from ..nodes.request import StartWorkerResponse, StartWorkerRequest
from ..nodes.request import StopWorkerRequest, StopWorkerResponse
from ..nodes.request import ListAgentsRequest, ListAgentsResponse
from ..nodes.request import ListWorkersRequest, ListWorkersResponse
from ..nodes.request import IdentifierRequest, IdentifierResponse
from ..nodes.request import ListNodesPrivRequest, ListNodesPrivResponse

REMOTE_MANAGER_ADDRESS = "remote_manager"


class RemoteManager:
    def __init__(self, node: LocalNode):
        self.node = node

    async def start(self):
        await self.node.start_internal_worker(REMOTE_MANAGER_ADDRESS, self)

    async def handle_message(self, context, message):
        try:
            request = decode_message(message)

            handlers = {
                StartAgentRequest: self.handle__start_agent,
                StartAgentsRequest: self.handle__start_agents,
                StartWorkerRequest: self.handle__start_worker,
                StopWorkerRequest: self.handle__stop_worker,
                ListAgentsRequest: self.handle__list_agents,
                ListWorkersRequest: self.handle__list_workers,
                IdentifierRequest: self.handle__identifier,
                ListNodesPrivRequest: self.handle__list_nodes_priv,
            }

            handler = handlers.get(type(request))
            if handler is not None:
                response = await handler(request)
            else:
                response = f"Unexpected Message: {request}"
        except Exception as e:
            response = str(e)

        if response is not None:
            response = encode_message(response)

            await context.reply(response)

    async def handle__identifier(self, _request: IdentifierRequest) -> IdentifierResponse:
        return IdentifierResponse("ok", await self.node.identifier())

    async def handle__start_agent(self, request: StartAgentRequest) -> StartAgentResponse:
        from ..agents import Agent

        await Agent.start(
            self.node,
            request.instructions,
            request.name,
            request.model,
            request.memory_model,
            request.memory_embeddings_model,
            request.tools,
            request.planner,
            request.exposed_as,
            request.knowledge,
            request.max_iterations,
        )

        return StartAgentResponse("ok")

    async def handle__start_agents(self, request: StartAgentsRequest) -> StartAgentsResponse:
        from ..agents import Agent

        agents = await Agent.start_many(
            self.node,
            request.instructions,
            request.number_of_agents,
            request.model,
            request.memory_model,
            request.memory_embeddings_model,
            request.tools,
            request.planner,
            request.knowledge,
            request.max_iterations,
        )

        names = [agent.name for agent in agents]

        return StartAgentsResponse("ok", names)

    async def handle__start_worker(self, request: StartWorkerRequest) -> StartWorkerResponse:
        request.worker.node = self.node
        await self.node.start_worker(request.name, request.worker, request.policy, request.exposed_as)

        return StartWorkerResponse("ok")

    async def handle__stop_worker(self, request: StopWorkerRequest):
        await self.node.stop_worker(request.name)

        return StopWorkerResponse("ok")

    async def handle__list_agents(self, _request: ListAgentsRequest):
        return ListAgentsResponse("ok", await self.node.list_agents())

    async def handle__list_workers(self, _request: ListWorkersRequest):
        return ListWorkersResponse("ok", await self.node.list_workers())

    async def handle__list_nodes_priv(self, _request: ListNodesPrivRequest):
        return ListNodesPrivResponse("ok", await self.node.list_nodes_priv())


class RemoteManagerClient:
    def __init__(self, node):
        self.node = node

    async def send_request(self, request):
        request = encode_message(request)

        # TODO: Policies
        response = await self.node.send_and_receive(REMOTE_MANAGER_ADDRESS, request)

        response = decode_message(response)

        if response.status != "ok":
            raise RuntimeError(f"Remote node response status: {response.status}")

        return response

    async def identifier(self):
        response = await self.send_request(IdentifierRequest())

        return response.identifier

    async def start_agent(
        self,
        instructions,
        name,
        model,
        memory_model,
        memory_embeddings_model,
        tools,
        planner,
        exposed_as,
        knowledge,
        max_iterations,
    ):
        await self.send_request(
            StartAgentRequest(
                instructions,
                name,
                model,
                memory_model,
                memory_embeddings_model,
                tools,
                planner,
                exposed_as,
                knowledge,
                max_iterations,
            )
        )

    async def start_agents(
        self,
        instructions,
        number_of_agents,
        model,
        memory_model,
        memory_embeddings_model,
        tools,
        planner,
        knowledge,
        max_iterations,
    ):
        response = await self.send_request(
            StartAgentsRequest(
                instructions,
                number_of_agents,
                model,
                memory_model,
                memory_embeddings_model,
                tools,
                planner,
                knowledge,
                max_iterations,
            )
        )

        return response.names

    async def start_worker(self, name, worker, policy, exposed_as):
        await self.send_request(StartWorkerRequest(name, worker, policy, exposed_as))

    async def stop_worker(self, name):
        await self.send_request(StopWorkerRequest(name))

    async def list_agents(self) -> list:
        response = await self.send_request(ListAgentsRequest())

        return response.agents

    async def list_workers(self) -> list:
        response = await self.send_request(ListWorkersRequest())

        return response.workers

    async def list_nodes_priv(self):
        response = await self.send_request(ListNodesPrivRequest())

        return response.nodes


def encode_message(message):
    message = dill.dumps(message)
    message = base64.b64encode(message).decode("utf-8")

    return message


def decode_message(message):
    message = base64.b64decode(message)
    message = dill.loads(message)

    return message
