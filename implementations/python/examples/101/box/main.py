import json

from ockam import Node


def generate_random_text():
    import random

    n = random.randint(100, 500)

    words = ["alpha", "beta", "gamma", "delta"]
    text = " ".join(random.choices(words, k=n))

    return text


DATA = {
    "0": {"id": "0", "name": "root", "children": ["ndas"]},
    "ndas": {"id": "ndas", "name": "ndas", "children": []},
}


def generate_data():
    import random
    import string

    n_folders = random.randint(5, 10)

    for i in range(n_folders):
        chars = string.ascii_letters + string.digits
        folder_name = "".join(random.choices(chars, k=10))
        folder_id = "".join(random.choices(chars, k=10))

        DATA["ndas"]["children"].append(folder_id)
        children = []

        n_files = random.randint(10, 20)
        for j in range(n_files):
            file_name = "".join(random.choices(chars, k=10))
            file_id = "".join(random.choices(chars, k=10))
            DATA[file_id] = {"id": file_id, "name": file_name, "text": generate_random_text()}
            children.append(file_id)

        DATA[folder_id] = {"name": folder_name, "children": children}


import re


def extract_text_id(s):
    match = re.fullmatch(r"text/(\w+)", s)
    if match:
        return match.group(1)
    return None


def extract_id(s):
    match = re.fullmatch(r"(\w+)", s)
    if match:
        return match.group(1)
    return None


class BoxWorker:
    async def handle_message(self, context, message):
        text_id = extract_text_id(message)
        if text_id:
            await context.reply(DATA[text_id]["text"])
        else:
            id = extract_id(message)

            response = []
            for id in DATA[id]["children"]:
                response.append({"name": DATA[id]["name"], "id": id})

            await context.reply(json.dumps(response))


async def main(node):
    generate_data()

    await node.start_worker("box", BoxWorker())


Node.start(main, http_server=None)
