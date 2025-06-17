from ockam import Agent, Flow, Node, START, END, TRY_AGAIN


async def main(node):
    triage = await Agent.start(
        node=node,
        name="language-triage",
        instructions="""
            You are a language triage agent.

            When you're given some input text, output if that input
            is in English, French, Spanish, or Hindi.

            Output only one lower case word from the list: `english`, `french`, `spanish`, `hindi`.

            Do not say anything else.
        """,
    )

    english_support = await Agent.start(
        node=node,
        name="english-assistant",
        instructions="You are an assistant who answers questions in English.",
    )

    french_support = await Agent.start(
        node=node,
        name="french-assistant",
        instructions="You are an assistant who answers questions in French.",
    )

    spanish_support = await Agent.start(
        node=node,
        name="spanish-assistant",
        instructions="You are an assistant who answers questions in Spanish.",
    )

    hindi_support = await Agent.start(
        node=node,
        name="hindi-assistant",
        instructions="""
            You are an assistant who answers questions in Hindi.
            You respond in hindi written in the latin alphabet.
        """,
    )

    english_quality_check = await Agent.start(
        node=node,
        name="english-quality-check",
        instructions=f"""
            You are an English quality check agent.

            If the input is correct English, has good grammar, and a friendly tone,
            output lower case string - `{TRY_AGAIN}`

            Otherwise, output lower case string - `continue`

            Don't say anything else other than `try again` or `continue`.
        """,
    )

    french_quality_check = await Agent.start(
        node=node,
        name="french-quality-check",
        instructions=f"""
            You are a French quality check agent.

            If the input is correct french, has good grammar, and a friendly tone,
            output lower case string - `{TRY_AGAIN}`

            Otherwise output lower case string - `continue`

            Don't say anything else other than `{TRY_AGAIN}` or `continue`.
        """,
    )

    spanish_quality_check = await Agent.start(
        node=node,
        name="spanish-quality-check",
        instructions=f"""
            You are a Spanish quality check agent.

            If the input is correct spanish, has good grammar, and a friendly tone,
            output lower case string - `{TRY_AGAIN}`

            Otherwise output lower case string - `continue`

            Don't say anything else other than `{TRY_AGAIN}` or `continue`.
        """,
    )

    hindi_quality_check = await Agent.start(
        node=node,
        name="hindi-quality-check",
        instructions=f"""
            You are a Hindi quality check agent.

            If the input is correct hindi, has good grammar, and a friendly tone,
            output lower case string - `{TRY_AGAIN}`

            Otherwise, output lower case string - `continue`

            Don't say anything else other than `{TRY_AGAIN}` or `continue`.
        """,
    )

    flow = Flow()

    flow.add(START, triage)
    flow.add(
        triage,
        {
            "english": english_support,
            "french": french_support,
            "spanish": spanish_support,
            "hindi": hindi_support,
        },
    )

    flow.add(english_support, english_quality_check)
    flow.add(
        english_quality_check,
        {
            TRY_AGAIN: english_support,
            "continue": END,
        },
    )

    flow.add(french_support, french_quality_check)
    flow.add(
        french_quality_check,
        {
            TRY_AGAIN: french_support,
            "continue": END,
        },
    )

    flow.add(spanish_support, spanish_quality_check)
    flow.add(
        spanish_quality_check,
        {
            TRY_AGAIN: spanish_support,
            "continue": END,
        },
    )

    flow.add(hindi_support, hindi_quality_check)
    flow.add(
        hindi_quality_check,
        {
            TRY_AGAIN: hindi_support,
            "continue": END,
        },
    )

    flow = await Flow.start(node, flow)
    print(flow.name)

    identifier = await flow.identifier()
    print(identifier)

    reply = await flow.send("where is the empire state building?")
    print(reply)

    reply = await flow.send("kya ho raha hai")
    print(reply)

    reply = await flow.send("que se passe-t-il")
    print(reply)

    reply = await flow.send("qué pasa")
    print(reply)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/00.py
