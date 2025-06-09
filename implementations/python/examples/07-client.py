from ockam import Agent, McpClient, McpTool, Model, Node, RemoteNode, info, HttpServer

import sys

"""
    This example shows how to:

    - Start two agents remotely on a node hosting an MCP server
    - Start a local agent that will use the two remote agents as tools, in order to answer requests.
"""


async def main(node):
    remote_node = RemoteNode(node, sys.argv[1])

    await Agent.start(
        node=remote_node,
        name="history-teacher",
        instructions="You are an assistant who is an expert in history.",
        model=Model(name="ollama_chat/llama3.2"),
        exposed_as="History assistant, ask anything about history.",
    )

    await Agent.start(
        node=remote_node,
        name="physics-teacher",
        instructions="You are an assistant who is an expert in Physics.",
        model=Model(name="ollama_chat/llama3.2"),
        exposed_as="Physics assistant, ask anything about Physics.",
    )

    agent = await Agent.start(
        node=node,
        name="Teacher",
        instructions="""
            You are a teacher.

            When you are given a question, decide if it is a
            history question or a physics question.

            Depending on the type of question, invoke the appropriate tool
            and explain its response.
        """,
        model=Model(name="ollama_chat/llama3.2"),
        tools=[
            McpTool("server-one", "history-teacher"),
            McpTool("server-one", "physics-teacher"),
        ],
    )

    reply = await agent.send("Who was Gandhi?")
    info(f"Who was Gandhi?\n\n{reply}")
    reply = await agent.send("What is dark matter?")
    info(f"What is dark matter?\n\n{reply}")


Node.start(
    main,
    mcp_clients=[
        McpClient(name="server-one", address="http://127.0.0.1:8000/sse"),
    ],
    wait_until_interrupted=False,
    http_server=HttpServer(listen_address="localhost:9001"),
)

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node1 ENROLLMENT_TICKET="$(ockam project ticket --relay node1 --attribute cluster=acme)" uv run examples/07-client.py node2
