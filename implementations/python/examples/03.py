from ockam import Model, Node, info

"""
    This example shows that, in addition to starting workers written in Python,
    Many different models can be invoked (this example just uses one, the full list is provided in models/model.py).
"""


class Echoer:
    async def handle_message(self, context, message):
        info(f"Echoer received: {message}")
        await context.reply(message)


async def main(node):
    model = Model(name="ollama_chat/llama3.2")
    response = await model.complete_chat(
        [{"content": "respond in 20 words. who are you?", "role": "user"}],
    )
    print(response)

    await node.start_worker("echoer", Echoer())
    reply = await node.send_and_receive("echoer", "hello")
    info(f"Reply received: {reply}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/03.py
