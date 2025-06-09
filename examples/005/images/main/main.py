from ockam import Agent, Memory, Model, Node


async def main(node):
    pioneer_docs = Memory("pioneer_ai_documents")
    await pioneer_docs.add_document(
        "Ownership in Pioneer.ai",
        "http://localhost:5555/ownership.md",
        content_type="text/markdown",
    )

    await Agent.start(
        node=node,
        name="henry",
        instructions="You are Henry, an expert legal assistant",
        model=Model("nova-micro-v1"),
        knowledge=pioneer_docs,
    )


Node.start(main)
