from ockam import Agent, Model, Node, SearchableKnowledge

"""
    This example shows how a model can be enriched with knowledge coming from inlined documents.
"""


async def main(node):
    restaurants = SearchableKnowledge(
        "restaurants",
        model=Model("ollama/nomic-embed-text"),
    )
    await restaurants.add_text(
        "Tony's Pizzeria Menu",
        """
        1. Margherita - $10
        2. Pepperoni - $12
        3. Hawaiian - $11
        4. Veggie - $9
        5. Four Cheese - $13
    """,
    )

    await restaurants.add_text(
        "Diner Menu",
        """
        1. Caesar Salad - $8
        2. Vegan Burger - $14
        3. Chicken - $15
        4. Shrimp Tacos - $16
        5. Chocolate Lava Cake - $7
    """,
    )

    agent = await Agent.start(
        node=node,
        name="Assistant",
        instructions="Assistant to solve some complex task ...",
        model=Model(name="ollama_chat/llama3.2"),
        knowledge=restaurants,
        max_knowledge_size=4096,
    )

    reply = await agent.send("What's the price of a pepperoni pizza?", scope="a", conversation="1")
    print(reply)

    reply = await agent.send("How do you know?", scope="a", conversation="1")
    print(reply)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/13.py
