from ockam import Agent, Model, Node, Tool
from datetime import datetime, UTC
from json import dumps
from ast import literal_eval

def run_python(source: str, locals_as_a_string: str) -> str:
    """
    Executes python code from a source string and provided local variables.

    The input source code should be written to store its final output
    as a string in a local variable named `result`.

    Args:
        source (str): The python code to execute.
        locals_as_a_string (str): A string representing the local variables dictionary.

    Returns:
        result (str): A string representing the result.
    """
    print(f"*** Running *** \n\n{source}\n\n{locals_as_a_string}\n\n***************", flush=True)
    locals = literal_eval(locals_as_a_string)
    exec(source, {}, locals)
    return locals["result"]


async def main(node):
    await Agent.start(
        node=node,
        name="chuck",
        instructions="You are chuck, an AI agent that can generate an run code in python.",
        model=Model("claude-sonnet-4-v1"),
        tools=[Tool(run_python)],
    )


Node.start(main)
