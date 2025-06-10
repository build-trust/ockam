from ockam import HttpServer, Node, Zone

from fastapi import FastAPI
from fastapi.responses import FileResponse, JSONResponse


class FileAnalyzerFactory:
    from ockam import LocalNodeProtocol

    @staticmethod
    def box_client():
        from box_sdk_gen import BoxClient, BoxCCGAuth, CCGConfig
        from os import environ

        return BoxClient(
            auth=BoxCCGAuth(
                config=CCGConfig(
                    client_id=environ["BOX_CLIENT_ID"],
                    client_secret=environ["BOX_CLIENT_SECRET"],
                    enterprise_id=environ["BOX_ENTERPRISE_ID"],
                )
            )
        )

    async def create(self, node: LocalNodeProtocol):
        return self.__class__.FileAnalyzer(node, self.__class__.box_client())

    class FileAnalyzer:
        def __init__(self, node, box_client):
            self.node = node
            self.box_client = box_client

        async def handle_message(self, context, message):
            from ockam import Agent, Model
            from box_ai_agents_toolkit import box_file_text_extract
            from asyncio import to_thread, sleep

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
                    content = await to_thread(
                        box_file_text_extract, self.box_client, file_id
                    )
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
                model=Model("nova-micro-v1"),
            )

            question = "Is the document missing a company name?\nRespond ONLY with YES or NO.\nDon't say anything else."
            message = f"File Name: {filename}\nFile Id: {file_id}\n\nFile Content:\n{content}\n\nQuestion:{question}\n\n"
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
    from ockam import LocalNodeProtocol

    @staticmethod
    def box_client():
        from box_sdk_gen import BoxClient, BoxCCGAuth, CCGConfig
        from os import environ

        return BoxClient(
            auth=BoxCCGAuth(
                config=CCGConfig(
                    client_id=environ["BOX_CLIENT_ID"],
                    client_secret=environ["BOX_CLIENT_SECRET"],
                    enterprise_id=environ["BOX_ENTERPRISE_ID"],
                )
            )
        )

    async def create(self, node: LocalNodeProtocol):
        return self.__class__.FolderAnalyzer(node, self.__class__.box_client())

    class FolderAnalyzer:
        def __init__(self, node, box_client):
            self.node = node
            self.box_client = box_client

        async def handle_message(self, context, message):
            import json

            folder = json.loads(message)

            files = self.box_client.folders.get_folder_items(folder).entries[:2]

            reply = []
            for file in files:
                reply.append(
                    {"folder": folder, "filename": file.name, "file_id": file.id}
                )

            await context.reply(json.dumps(reply))


class FoldersAnalyzerFactory:
    from ockam import LocalNodeProtocol

    @staticmethod
    def box_client():
        from box_sdk_gen import BoxClient, BoxCCGAuth, CCGConfig
        from os import environ

        return BoxClient(
            auth=BoxCCGAuth(
                config=CCGConfig(
                    client_id=environ["BOX_CLIENT_ID"],
                    client_secret=environ["BOX_CLIENT_SECRET"],
                    enterprise_id=environ["BOX_ENTERPRISE_ID"],
                )
            )
        )

    async def create(self, node: LocalNodeProtocol):
        return self.__class__.FoldersAnalyzer(node, self.__class__.box_client())

    class FoldersAnalyzer:
        def __init__(self, node, box_client):
            self.node = node
            self.box_client = box_client

        async def handle_message(self, context, _message):
            import json

            folders = []
            for i in self.box_client.folders.get_folder_items("0").entries:
                if i.name == "ndas":
                    for j in self.box_client.folders.get_folder_items(i.id).entries:
                        folders.append(j.id)

            await context.reply(json.dumps(folders))


async def analyze(node):
    from ockam import Squad
    from ockam.squads.squad import SquadStep, PerNode, PerItem

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

    async def list_per_runner(runner):
        workers_on_this_runner = await runner.list_workers()
        return {runner.name: [w["name"] for w in workers_on_this_runner]}

    futures = [list_per_runner(runner) for runner in runners]
    workers_per_runner = await ockam.gather(*futures, timeout=5, return_exceptions=True)

    # ignore errors
    workers_per_runner = [r for r in workers_per_runner if not isinstance(r, Exception)]
    # flatten
    workers_per_runner = {k: v for d in workers_per_runner for k, v in d.items()}

    return workers_per_runner


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
            return JSONResponse(content=workers)

        @self.api.get("/runners")
        async def get_runners():
            runners = await Zone.nodes(node, filter="runner")
            return JSONResponse(content={"runners": [r.name for r in runners]})


Node.start(http_server=HttpServer(api=Api()), cache_secure_channels=True)
