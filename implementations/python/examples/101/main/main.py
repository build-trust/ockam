from ockam import HttpServer, Node, Zone, Squad, LocalNodeProtocol

from fastapi import FastAPI
from fastapi.responses import FileResponse, JSONResponse

from sys import argv

from ockam.squads.squad import SquadStep, PerNode, PerItem


class FileAnalyzerFactory:
    async def create(self, node: LocalNodeProtocol):
        from ockam.nodes.box import BoxClient

        box_client = BoxClient(node)
        return self.__class__.FileAnalyzer(node, box_client)

    class FileAnalyzer:
        def __init__(self, node, box_client):
            self.node = node
            self.box_client = box_client

        async def handle_message(self, context, message):
            from ockam import Agent, Model
            from asyncio import sleep

            import json
            import random

            j = json.loads(message)
            folder = j["folder"]
            file_id = j["file_id"]
            filename = j["filename"]

            max_attempts = 20
            delay = 60  # seconds
            attempt = 0
            content = None

            while attempt < max_attempts:
                try:
                    content = await self.box_client.get_text(file_id)
                    break
                except Exception:
                    attempt += 1
                    if attempt == max_attempts:
                        raise
                    jitter = random.uniform(0, delay * 0.3)  # add up to 30% jitter
                    await sleep(delay + jitter)
                    delay *= 2  # exponential backoff

            agent = await Agent.start(
                node=self.node,
                instructions="You are an agent who answers questions about a provided file.",
                model=Model("ollama_chat/llama3.2"),
            )

            question = "Is the document missing a company name?\nRespond ONLY with YES or NO.\nDon't say anything else."
            message = (
                f"File Name: {filename}\nFile Id: {file_id}\n\nFile Content:\n{content}\n\nQuestion:{question}\n\n"
            )
            analysis = await agent.send(message, timeout=1000)

            await Agent.stop(self.node, agent.name)

            reply = json.dumps(
                {
                    "folder": folder,
                    "filename": filename,
                    "file_id": file_id,
                    "analysis": analysis[0].content,
                }
            )

            await context.reply(reply)


class FolderAnalyzerFactory:
    async def create(self, node: LocalNodeProtocol):
        from ockam.nodes.box import BoxClient

        box_client = BoxClient(node)
        return self.__class__.FolderAnalyzer(node, box_client)

    class FolderAnalyzer:
        def __init__(self, node, box_client):
            self.node = node
            self.box_client = box_client

        async def handle_message(self, context, message):
            import json

            folder = json.loads(message)

            files = await self.box_client.folders.get_folder_items(folder)

            reply = []
            for f in files:
                reply.append({"folder": folder, "file_id": f["id"], "filename": f["name"]})

            await context.reply(json.dumps(reply))


class FoldersAnalyzerFactory:
    async def create(self, node: LocalNodeProtocol):
        from ockam.nodes.box import BoxClient

        box_client = BoxClient(node)
        return self.__class__.FoldersAnalyzer(node, box_client)

    class FoldersAnalyzer:
        def __init__(self, node, box_client):
            self.node = node
            self.box_client = box_client

        async def handle_message(self, context, _message):
            import json

            folders = []
            items = await self.box_client.folders.get_folder_items("0")
            for i in items:
                if i["name"] == "ndas":
                    items = await self.box_client.folders.get_folder_items(i["id"])
                    for j in items:
                        folders.append(j["id"])

            await context.reply(json.dumps(folders))


async def analyze(node):
    runners = await Zone.nodes(node, filter="runner")

    return await Squad.run(
        node,
        runners,
        "",
        [
            SquadStep("get_folders", FoldersAnalyzerFactory(), PerNode()),
            SquadStep("get_list_of_files", FolderAnalyzerFactory(), PerItem()),
            SquadStep("analyze_file", FileAnalyzerFactory()),
        ],
    )


async def list_workers(node):
    import ockam

    runners = await Zone.nodes(node, filter="runner")
    futures = [runner.list_workers() for runner in runners]
    workers_per_runner = await ockam.gather(*futures)
    workers = []
    for runner, workers_on_this_runner in zip(runners, workers_per_runner):
        for w in workers_on_this_runner:
            workers.append({"worker_name": w["name"], "runner_name": runner.name})
    return workers


class Api:
    def __init__(self):
        self.api = FastAPI()

    def routes(self, node):
        @self.api.get("/")
        async def index():
            return FileResponse("index.html")

        @self.api.post("/analyze")
        async def post_analyze():
            response = await analyze(node)
            return JSONResponse(content=response)

        @self.api.get("/runners/workers")
        async def get_workers():
            workers = await list_workers(node)
            return JSONResponse(content={"workers": workers})

        @self.api.get("/runners")
        async def get_runners():
            runners = await Zone.nodes(node, filter="runner")
            return JSONResponse(content={"runners": [r.name for r in runners]})


Node.start(http_server=HttpServer(listen_address=argv[1], api=Api()))
