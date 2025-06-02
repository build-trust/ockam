from ockam import Agent, Node, HttpServer, Repl, Model
from api_15 import Api

"""
  This example shows how a HTTP server can be started to interact with some agents deployed on a node.
  An API with custom routes can also be defined.

  Pre-requisites:

  To run this example locally you need to start:

  - `sh> ollama serve > /dev/null 2>&1`
  - `sh> uv run examples/15.py`

  Example queries:

  # retrieve the list of all agents
  http localhost:8000/agents

  # send a request to a specific agent
  http POST localhost:8000/agents/henry message="What is a LBO?"

  # retrieve the list of all tools
  http localhost:8000/tools

  # retrieve the global conversations for a given agent
  http localhost:8000/agents/henry/conversations

  # retrieve the global conversations for a given agent and a given scope
  http localhost:8000/agents/henry/scopes/acme/conversations

  # retrieve the conversation for a given agent, scope and conversation id
  http localhost:8000/agents/henry/scopes/acme/conversations/1

  # call the custom API
  http POST localhost:8000/analysis?network=acme

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


Node.start(main, http_server=HttpServer(api=Api()))
