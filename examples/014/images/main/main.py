from ockam import Agent, Model, Node, set_log_level


async def main(node):
    model = Model(name="llama3.3", max_input_tokens=150)
    memory_model = Model(name="llama3.3")
    memory_embeddings_model = Model(name="titan-embed-text-v2")

    await Agent.start(
        node=node,
        name="assistant",
        instructions="You are an assistant who acts like a friend and can maintain the conversation on any topic.",
        model=model,
        memory_model=memory_model,
        memory_embeddings_model=memory_embeddings_model,
    )


set_log_level("mem0", "DEBUG")
set_log_level("agent", "INFO")

Node.start(main)
