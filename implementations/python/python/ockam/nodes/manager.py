import dill
import base64

from ..nodes.request import StartAgentResponse, StartAgentRequest
from ..nodes.request import StartAgentsResponse, StartAgentsRequest
from ..nodes.request import StartWorkerResponse, StartWorkerRequest
from ..nodes.request import StopWorkerRequest, StopWorkerResponse
from ..nodes.request import ListAgentsRequest, ListAgentsResponse
from ..nodes.request import ListWorkersRequest, ListWorkersResponse

REMOTE_MANAGER_ADDRESS = "remote_manager"


class RemoteManager:
    def __init__(self, node):
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

    async def handle__start_agent(self, request: StartAgentRequest) -> StartAgentResponse:
        from ..agents import Agent

        await Agent.start(
            self.node,
            request.instructions,
            request.name,
            request.model,
            request.tools,
            request.planner,
            request.exposed_as,
            request.knowledge,
            request.max_knowledge_size,
        )

        return StartAgentResponse("ok")

    async def handle__start_agents(self, request: StartAgentsRequest) -> StartAgentsResponse:
        from ..agents import Agent

        agents = await Agent.start_many(
            self.node,
            request.instructions,
            request.number_of_agents,
            request.model,
            request.tools,
            request.planner,
        )

        names = [agent.name for agent in agents]

        return StartAgentsResponse("ok", names)

    async def handle__start_worker(self, request: StartWorkerRequest) -> StartWorkerResponse:
        request.worker.node = self.node
        self.node.start_worker(request.name, request.worker, request.policy)

        return StartWorkerResponse("ok")

    async def handle__stop_worker(self, request: StopWorkerRequest):
        self.node.stop_worker(request.name)

        return StopWorkerResponse("ok")

    async def handle__list_agents(self, _request: ListAgentsRequest):
        agents = self.node.list_agents()

        return ListAgentsResponse("ok", {"agents": agents})

    async def handle__list_workers(self, _request: ListWorkersRequest):
        workers = await self.node.list_workers()

        return ListWorkersResponse("ok", {"workers": workers})


class RemoteManagerClient:
    def __init__(self, node):
        self.node = node

    async def start_agent(self, instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size):
        request = encode_message(
            StartAgentRequest(instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size)
        )

        response = await self.node.send_and_receive(destination=REMOTE_MANAGER_ADDRESS, message=request)

        response = decode_message(response)

        if response.status != "ok":
            raise RuntimeError(f"Remote node start_agent status: {response.status}")

    async def start_agents(self, instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size):
        request = encode_message(
            StartAgentsRequest(instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size)
        )

        response = await self.node.send_and_receive(destination=REMOTE_MANAGER_ADDRESS, message=request)

        response = decode_message(response)

        if response.status != "ok":
            raise RuntimeError(f"Remote node start_agents status: {response.status}")

        return response.names

    async def start_worker(self, name, worker, policy):
        request = encode_message(StartWorkerRequest(name, worker, policy))

        response = await self.node.send_and_receive(destination=REMOTE_MANAGER_ADDRESS, message=request)

        response = decode_message(response)

        if response.status != "ok":
            raise RuntimeError(f"Remote node start_worker status: {response.status}")

    async def stop_worker(self, name):
        request = encode_message(StopWorkerRequest(name))

        response = await self.node.send_and_receive(destination=REMOTE_MANAGER_ADDRESS, message=request)

        response = decode_message(response)

        if response.status != "ok":
            raise RuntimeError(f"Remote node stop_agent status: {response.status}")

    async def list_agents(self) -> list:
        request = encode_message(ListAgentsRequest())

        response = await self.node.send_and_receive(destination=REMOTE_MANAGER_ADDRESS, message=request)

        response = decode_message(response)

        if response.status != "ok":
            raise RuntimeError(f"Remote node list_agents status: {response.status}")

        return response.agents["agents"]

    async def list_workers(self) -> list:
        request = encode_message(ListWorkersRequest())

        response = await self.node.send_and_receive(destination=REMOTE_MANAGER_ADDRESS, message=request)

        response = decode_message(response)

        if response.status != "ok":
            raise RuntimeError(f"Remote node list_workers status: {response.status}")

        return response.workers["workers"]


def encode_message(message):
    message = dill.dumps(message)
    message = base64.b64encode(message).decode("utf-8")

    return message


def decode_message(message):
    message = base64.b64decode(message)
    message = dill.loads(message)

    return message
