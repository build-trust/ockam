from ockam import Agent, Flow, FlowOperation, Node, START, END, Repl, Model

from sys import argv


async def main(node):
    triage = await Agent.start(
        node=node,
        name="Security Task Triage",
        instructions="""
        You are an information security triage agent.

        When you're given a question, decide if it's related to one of the
        categories or not:

        - `network`: vulnerabilities in networks, internet, cloud, etc.
        - `code`: vulnerabilities in code snippets

        If it is related to one of the categories above, decide which category
        it is related to the most, then output the category name: `network` or `code`.

        If it is not related to any of the categories above - output `other`.

        Do not say anything else other than the category `network` or `code`, or `other`.

        Examples:
            ```
            Question: is it safe to open all ports on a server?
            Answer: `network`
            ```

            ```
            Question:
                ```
                print("Hello world!")
                ```
            Answer: `code`
            ```

            ```
            Question: why the sky is blue?
            Answer: `other`
            ```
        """,
    )

    network_expert = await Agent.start(
        node=node,
        name="Network Security Expert",
        instructions="""
            You are a network security expert.
            Answer network security questions.
        """,
    )

    code_security_fixer = await Agent.start(
        node=node,
        name="Code Security Fixer",
        instructions="""
            You are a codding expert who knows the Python programming language.

            You're given a code snippet and information about a vulnerability in
            it. Change the code snippet precisely to fix that vulnerability.

            Don't change anything else.
            Only print the fixed code snippet.
            Under no circumstances print anything else except the changed code snippet.
            """,
    )

    code_evaluator = await Agent.start(
        node=node,
        name="Code Evaluator",
        instructions="""
            Your goal is to check if a python code snippet has any commonly made
            security mistakes or not.

            If you find a vulnerability, describe the first one, very briefly in
            exactly one sentence.

            Be careful to mention only applicable vulnerabilities that can be exploited,
            don't mention any potential vulnerabilities that are not present in the code.

            Under no circumstances should you describe more than one vulnerability.

            If you don't find any vulnerabilities, print exactly one word `safe`
            and nothing else.

            Examples:
                ```
                Question:
                    ```
                    filename = input("Enter file to open: ")
                    with open(f"/safe/dir/{{filename}}") as f:
                        data = f.read()
                    ```
                Answer: user can exploit filename to open arbitrary files.
                ```

                ```
                Question:
                    ```
                    API_KEY = "12345-SECRET-KEY"
                    ```
                Answer: Hardcoded secrets are not safe.
                ```

                ```
                Question:
                    ```
                    import subprocess
                    filename = input("Enter file name to list: ")
                    subprocess.run(["ls", filename])
                    ```
                Answer: `safe`
                ```

                ```
                Question:
                    ```
                    try:
                        age = int(input("Enter your age: "))
                        if age < 0:
                            raise ValueError("Age can't be negative")
                    except ValueError:
                        print("Invalid age input.")
                    ```
                Answer: `safe`
                ```

                ```
                Question:
                    ```
                    import ast
                    try:
                        result = ast.literal_eval(user_input)
                    except (ValueError, SyntaxError):
                        print("Invalid expression")
                    ```
                Answer: `safe`
                ```

                ```
                Question:
                    ```
                    import random
                    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

                    private_bytes = bytes([random.randint(0, 255) for _ in range(32)])
                    private_key = Ed25519PrivateKey.from_private_bytes(private_bytes)

                    # Sign a message
                    message = b"Sensitive data to sign"
                    signature = private_key.sign(message)

                    # Verify (just to show it's valid with the key pair)
                    public_key = private_key.public_key()
                    public_key.verify(signature, message)

                    print("Signature:", signature.hex())
                    ```
                Answer: cryptographical keys should be generated only using
                cryptographically secure randomness sources.
                ```

                ```
                Question:
                    ```
                    import sqlite3

                    conn = sqlite3.connect('example.db')
                    cursor = conn.cursor()

                    cursor.execute("INSERT INTO users (username, password) VALUES (?, ?)", (username, password))
                    conn.commit()

                    # Clean up
                    cursor.close()
                    conn.close()
                    ```
                Answer: `safe`
                ```
            """,
    )

    bouncer = await Agent.start(
        node=node,
        name="Polite Bouncer",
        instructions="""
            You MUST politely reject the user question explaining that only
            security or networking questions can be asked. Be terse and concise.

            For example:
            ```
            Question: What is the meaning of life?
            Answer: I'm sorry, but I can only answer questions related to security or networking.
            ```
        """,
    )

    flow = Flow()

    flow.add(START, triage)

    flow.add(triage, network_expert, condition="network")
    flow.add(triage, bouncer, condition="other")
    flow.add(triage, code_evaluator, condition="code", operation=FlowOperation.EVALUATE)

    flow.add(network_expert, END)
    flow.add(bouncer, END)

    flow.add(code_evaluator, code_security_fixer)
    flow.add(code_evaluator, END, condition="safe")
    flow.add(code_security_fixer, code_evaluator, operation=FlowOperation.EVALUATE)

    flow = await Flow.start(node, flow)

    await Repl.start(flow, "localhost:7000")


Node.start(main)
