from ockam import HttpServer, Node, Zone
from box_sdk_gen import BoxClient, BoxCCGAuth, CCGConfig

from fastapi import FastAPI
from fastapi.responses import FileResponse, JSONResponse

from os import environ
from sys import argv

import json
import secrets


class FoldersAnalyzer:
    class FolderAnalyzer:
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
                file_id = j["file_id"]
                filename = j["filename"]

                max_attempts = 20
                delay = 60  # seconds
                attempt = 0
                content = None

                while attempt < max_attempts:
                    try:
                        content = await to_thread(box_file_text_extract, self.box_client, file_id)
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
                        "filename": filename,
                        "file_id": file_id,
                        "analysis": analysis[0].content,
                    }
                )

                await context.reply(reply)

        def __init__(self, node, box_client):
            self.node = node
            self.box_client = box_client

        async def handle_message(self, context, message):
            import ockam
            import json
            import secrets

            folder = json.loads(message)

            files = self.box_client.folders.get_folder_items(folder).entries[:10]

            async def start_file_analysis(file):
                worker_name = secrets.token_hex(6)
                await self.node.start_worker(worker_name, self.__class__.FileAnalyzer(self.node, self.box_client))
                mailbox_name = secrets.token_hex(6)
                mailbox = await self.node.create_mailbox(mailbox_name)

                await mailbox.send(destination=worker_name,
                                   message=json.dumps({"filename": file.name, "file_id": file.id}))
                return worker_name, mailbox

            futures = [start_file_analysis(file) for file in files]
            handles = await ockam.gather(*futures, batch_size=50)

            async def finish_file_analysis(name, mailbox):
                analysis = await mailbox.receive(timeout=1000)
                await self.node.stop_worker(name)
                return analysis

            futures = [finish_file_analysis(worker_name, mailbox) for worker_name, mailbox in
                       handles]
            analyses = await ockam.gather(*futures, batch_size=100)

            await context.reply(json.dumps({"folder": folder, "analyses": analyses}))

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

    async def handle_message(self, context, message):
        import json
        import ockam
        import secrets

        folders = json.loads(message)

        box_client = self.__class__.box_client()

        async def start_folder_analysis(folder):
            worker_name = secrets.token_hex(6)
            await self.node.start_worker(worker_name, self.__class__.FolderAnalyzer(self.node, box_client))
            mailbox_name = secrets.token_hex(6)
            mailbox = await self.node.create_mailbox(mailbox_name)

            await mailbox.send(destination=worker_name, message=json.dumps(folder))

            return worker_name, mailbox

        futures = [start_folder_analysis(folder) for folder in folders]
        handles = await ockam.gather(*futures, batch_size=10)

        async def finish_folder_analysis(worker_name, mailbox):
            analysis = await mailbox.receive(timeout=1000)
            await self.node.stop_worker(worker_name)
            return analysis

        futures = [finish_folder_analysis(worker_name, mailbox) for worker_name, mailbox in
                   handles]
        analyses = await ockam.gather(*futures, batch_size=100)

        await context.reply(json.dumps(analyses))


async def run_analyzer(node, folders):
    worker_name = secrets.token_hex(6)
    await node.start_worker(worker_name, FoldersAnalyzer())

    folders_json = json.dumps(folders)
    reply_json = await node.send_and_receive(worker_name, folders_json, timeout=1000)
    reply = json.loads(reply_json)

    await node.stop_worker(worker_name)
    return reply


async def analyze(node):
    import ockam

    runners = await Zone.nodes(node, filter="runner")
    folders = await list_folders()

    num_runners = len(runners)
    if num_runners == 0:
        return []
    if num_runners > len(folders):
        runners = runners[: len(folders)]
        num_runners = len(runners)

    folders = split_list_into_n_parts(folders, num_runners)
    futures = [run_analyzer(runner, folders[i]) for i, runner in enumerate(runners)]
    replies = await ockam.gather(*futures)
    # TODO: Should replies be flatten?

    return replies


def split_list_into_n_parts(lst, n):
    q, r = divmod(len(lst), n)
    return [lst[i * q + min(i, r): (i + 1) * q + min(i + 1, r)] for i in range(n)]


async def list_folders():
    client = BoxClient(
        auth=BoxCCGAuth(
            config=CCGConfig(
                client_id=environ["BOX_CLIENT_ID"],
                client_secret=environ["BOX_CLIENT_SECRET"],
                enterprise_id=environ["BOX_ENTERPRISE_ID"],
            )
        )
    )

    folders = []
    for i in client.folders.get_folder_items("0").entries:
        if i.name == "ndas":
            for j in client.folders.get_folder_items(i.id).entries:
                folders.append(j.id)

    return folders


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


Node.start(http_server=HttpServer(api=Api()), cache_secure_channels=True)
