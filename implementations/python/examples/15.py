from ockam import Agent, Node, HttpServer, Repl, Model
from app_15 import app

"""
    This example shows how a HTTP server can be started to interact with some agents deployed on a node.
    An API with custom routes is defined and adds a mandatory API key header for authentication.

    Pre-requisites:

    To run this example locally you need to start:

    - `sh> ollama serve > /dev/null 2>&1`
    - `sh> export API_KEY=ockam`
    - `sh> uv run examples/15.py`

    Example queries:

    # retrieve the list of all agents
    http localhost:8000/agents x-api-key:ockam

    # send a request to a specific agent
    http POST localhost:8000/agents/henry message="What is a LBO?" x-api-key:ockam

    # retrieve the list of all tools
    http localhost:8000/tools x-api-key:ockam

    # retrieve the global conversations for a given agent
    http localhost:8000/agents/henry/conversations x-api-key:ockam

    # retrieve the global conversations for a given agent and a given scope
    http localhost:8000/agents/henry/scopes/acme/conversations x-api-key:ockam

    # retrieve the conversation for a given agent, scope and conversation id
    http localhost:8000/agents/henry/scopes/acme/conversations/1 x-api-key:ockam

    # call the custom API
    http POST localhost:8000/analysis?network=acme x-api-key:ockam
"""


async def main(node):
    agent = await Agent.start(
        name="henry",
        node=node,
        model=Model(name="llama3.2"),
        instructions="""
            You are Henry, an expert legal assistant.
            You have in-depth knowledge of United States corporate law.
        """,
    )
    await Repl.start(agent)


Node.start(main, http_server=HttpServer(app=app))
