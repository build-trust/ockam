from ockam import Agent, Flow, Node, START, END, Repl, FlowOperation


async def main(node):
    triage = await Agent.start(
        node=node,
        name="Security Task Triage",
        instructions="""
You are an information security agent.

When you're given a question, decide if it's related to one of the categories or not:
    - `network`: vulnerabilities in networks, internet, cloud, etc.
    - `code`: vulnerabilities in code snippets

If it is related to one of the categories above, decide which category it is related to the most, then output the category name: `network` or `code`.
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
        instructions="You are a network security expert. Answer network security questions.",
    )

    code_security_expert = await Agent.start(
        node=node,
        name="Code Security Expert",
        instructions="""
You are a codding expert who knows the Python programming language.

You're given a code snippet and information about a vulnerability in it. Change the code snippet precisely to fix that vulnerability. Don't change anything else.
Only print the fixed code snippet. Under no circumstances print anything else except the changed code snippet.
""",
    )

    code_evaluator = await Agent.start(
        node=node,
        name="Code Evaluator",
        instructions="""
Your goal is to figure out if a python code snippet has any vulnerabilities.

If you found vulnerabilities, describe the first one very briefly in exactly one sentence.
Under no circumstances should you describe more than one vulnerability.
If you haven't found any vulnerabilities, print exactly one word `safe` and nothing else.

Examples:
    ```
    Question:
        ```
        filename = input("Enter file to open: ")
        with open(f"/safe/dir/{filename}") as f:
            data = f.read()
        ```
    Answer: user can exploit filename to open arbitrary files.
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
""",
    )

    flow = Flow()

    # TODO: Remove START
    # TODO: Support condition in form of invokable function
    flow.add(START, triage)

    flow.add(triage, network_expert, condition="network")
    flow.add(network_expert, END)

    flow.add(triage, code_evaluator, condition="code", operation=FlowOperation.EVALUATE)
    flow.add(code_evaluator, code_security_expert)
    flow.add(code_evaluator, END, condition="safe")
    flow.add(code_security_expert, code_evaluator, operation=FlowOperation.EVALUATE)

    flow = await Flow.start(node, flow)

    reply = await flow.send("is it safe to allow traffic from `0.0.0.0/0` on all ec2 machines on AWS?")
    print(reply)

    reply = await flow.send("""
```
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
import os
import sqlite3
import random

# Setup database and table
conn = sqlite3.connect(':memory:')  # In-memory DB for quick test
cursor = conn.cursor()
cursor.execute("CREATE TABLE users (username TEXT, password TEXT)")
cursor.execute("INSERT INTO users VALUES ('admin', 'securepass')")

# User input
username = input("Enter username: ")
password = input("Enter password: ")

query = f"SELECT * FROM users WHERE username = '{username}' AND password = '{password}'"
print("Running query:", query)
cursor.execute(query)

# Fetch results
if cursor.fetchone():
    print("Login successful!")
else:
    print("Login failed.")

cursor.close()
conn.close()

# Generate a cryptographic AES key
aes_key = bytes([random.randint(0, 255) for _ in range(32)])

# AES-GCM encryption setup
aesgcm = AESGCM(aes_key)
nonce = os.urandom(12)
data = b"This is a secret"
aad = None

ciphertext = aesgcm.encrypt(nonce, data, aad)
decrypted = aesgcm.decrypt(nonce, ciphertext, aad)

print("Ciphertext:", ciphertext.hex())
print("Decrypted:", decrypted.decode())
```
""")

    print(reply)

    await Repl.start(flow)


Node.start(main)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/00.py
