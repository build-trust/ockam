class IdentifierRequest:
    pass


class IdentifierResponse:
    def __init__(self, status, identifier):
        self.status = status
        self.identifier = identifier


class StartAgentRequest:
    def __init__(
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
        self.instructions = instructions
        self.name = name
        self.model = model
        self.memory_model = memory_model
        self.memory_embeddings_model = memory_embeddings_model
        self.tools = tools
        self.planner = planner
        self.exposed_as = exposed_as
        self.knowledge = knowledge
        self.max_iterations = max_iterations


class StartAgentResponse:
    def __init__(self, status):
        self.status = status


class StartAgentsRequest:
    def __init__(
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
        self.instructions = instructions
        self.number_of_agents = number_of_agents
        self.model = model
        self.memory_model = memory_model
        self.memory_embeddings_model = memory_embeddings_model
        self.tools = tools
        self.planner = planner
        self.knowledge = knowledge
        self.max_iterations = max_iterations


class StartAgentsResponse:
    def __init__(self, status, names):
        self.status = status
        self.names = names


class StartWorkerRequest:
    def __init__(self, name, worker, policy, exposed_as):
        self.name = name
        self.worker = worker
        self.policy = policy
        self.exposed_as = exposed_as


class StartWorkerResponse:
    def __init__(self, status):
        self.status = status


class StopWorkerRequest:
    def __init__(self, name):
        self.name = name


class StopWorkerResponse:
    def __init__(self, status):
        self.status = status


class ListAgentsRequest:
    pass


class ListAgentsResponse:
    def __init__(self, status, agents):
        self.status = status
        self.agents = agents


class ListWorkersRequest:
    pass


class ListWorkersResponse:
    def __init__(self, status, workers):
        self.status = status
        self.workers = workers


class ListNodesPrivRequest:
    pass


class ListNodesPrivResponse:
    def __init__(self, status, nodes):
        self.status = status
        self.nodes = nodes
