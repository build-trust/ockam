from ockam import Agent, Model, Node, SearchableKnowledge

"""
  This example shows how a model can be enriched with knowledge coming from documents retrieved online.
"""


async def main(node):
    ockam_documentation = SearchableKnowledge(
        "ockam_documentation",
        model=Model("ollama/nomic-embed-text"),
    )

    base_url = "https://raw.githubusercontent.com/build-trust/ockam-documentation/refs/heads/main"
    documents = [
        "README.md",
        "reference/command/README.md",
        "reference/command/credentials.md",
        "reference/command/identities.md",
        "reference/command/secure-channels.md",
        "reference/command/advanced-routing.md",
    ]

    for document in documents:
        await ockam_documentation.add_document(
            document,
            f"{base_url}/{document}",
            content_type="text/markdown",
        )

    agent = await Agent.start(
        node=node,
        name="Assistant",
        instructions="Assistant to solve some complex task ...",
        model=Model(name="ollama_chat/llama3.2"),
        knowledge=ockam_documentation,
        max_knowledge_size=4096,
    )

    reply = await agent.send("What's ockam?", scope="a", conversation="1")
    print(reply)

    reply = await agent.send("What's a relay, and when should I use it?", scope="a", conversation="1")
    print(reply)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/14.py
