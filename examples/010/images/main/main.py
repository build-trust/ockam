from ockam import HttpServer, Node, Zone

from fastapi import FastAPI
from fastapi.responses import FileResponse, JSONResponse

import asyncio
import secrets
import json


class CodeAnalyzer:
    def files_from_github_repo(self, org, repo, branch="main"):
        import os
        import shutil
        import urllib.request
        import zipfile

        url = f"https://github.com/{org}/{repo}/archive/refs/heads/{branch}.zip"
        zip_path = f"/tmp/{repo}-{branch}.zip"
        urllib.request.urlretrieve(url, zip_path)

        extracted = f"/tmp/{repo}-{branch}"
        if os.path.exists(extracted):
            shutil.rmtree(extracted)

        os.makedirs(extracted)
        with zipfile.ZipFile(zip_path, "r") as zip_ref:
            zip_ref.extractall(extracted)

        os.remove(zip_path)

        files = []
        for root, dirs, file_list in os.walk(extracted):
            for file in file_list:
                full_path = os.path.join(root, file)
                stat = os.stat(full_path)
                if file.endswith(".py") and stat.st_size < 1024 * 50:
                    with open(full_path, "r", encoding="utf-8") as f:
                        content = f.read()
                        relative_path = os.path.relpath(full_path, extracted)
                        relative_path = "./" + relative_path[len(f"{repo}-{branch}/") :]
                        files.append((relative_path, content))

        shutil.rmtree(extracted)
        return files

    async def analyze_file(self, filename, content):
        from ockam import Agent, Model

        agent = await Agent.start(
            node=self.node,
            instructions="""
                You are an expert in programming with Python and writing secure code.
                When you're given a code snippet you decide if it is secure or not.

                If it is secure output - YES
                If it is not secure output - NO

                Don't say anything else. Only output one upper case word YES or NO.
            """,
            model=Model("llama3.1-8b-instruct"),
        )

        message = f"Filename: {filename}\nContent:\n\n{content}"
        receiver = await agent.send_message(message)
        return (agent, filename, receiver)

    async def collect_results(self, agent, filename, receiver):
        from ockam import Agent

        analysis = await receiver.receive_message(timeout=1000)
        _ = await Agent.stop(self.node, agent.name)
        return {"file": filename, "analysis": analysis[0].content}

    async def analyze_github_repo(self, org, repo, branch="main"):
        from ockam import gather
        import asyncio

        name = f"{org}/{repo}/{branch}"
        print(name, flush=True)

        try:
            files = await asyncio.to_thread(self.files_from_github_repo, org, repo, branch)

            futures = [self.analyze_file(f, c) for f, c in files]
            agents = await gather(*futures, batch_size=100)

            futures = [self.collect_results(a, f, r) for (a, f, r) in agents]
            analyses = await gather(*futures, batch_size=100)

            return {"repo": name, "number_of_files": len(files), "analysis": analyses}
        except Exception as e:
            return {"repo": name, "error": str(e)}

    async def handle_message(self, context, message):
        import json
        import asyncio

        repos = json.loads(message)

        futures = []
        for repo in repos:
            org_name, repo_name = repo.split("/")
            f = self.analyze_github_repo(org_name, repo_name)
            futures.append(f)
        analyses = await asyncio.gather(*futures)

        reply = json.dumps(analyses)
        await context.reply(reply)


async def run_analyzer(node, repos):
    name = secrets.token_hex(3)
    await node.start_worker(name, CodeAnalyzer())

    repos_json = json.dumps(repos)
    reply_json = await node.send_and_receive(name, repos_json, timeout=1000)
    reply = json.loads(reply_json)

    await node.stop_worker(name)
    return reply


async def analyze(node, repos):
    runners = await Zone.nodes(node, filter="runner")

    num_runners = len(runners)
    if num_runners == 0:
        return []
    if num_runners > len(repos):
        runners = runners[: len(repos)]
        num_runners = len(runners)

    repos = split_list_into_n_parts(repos, num_runners)
    futures = [run_analyzer(runner, repos[i]) for i, runner in enumerate(runners)]
    replies = await asyncio.gather(*futures)

    return replies


def split_list_into_n_parts(lst, n):
    q, r = divmod(len(lst), n)
    return [lst[i * q + min(i, r) : (i + 1) * q + min(i + 1, r)] for i in range(n)]


async def list_workers(node):
    runners = await Zone.nodes(node, filter="runner")
    futures = [runner.list_workers() for runner in runners]
    workers_per_runner = await asyncio.gather(*futures)
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
            repos = [
                "pallets/flask",
                "pallets/click",
                "pallets/werkzeug",
                "pallets/jinja",
                "pallets/markupsafe",
                "pallets/itsdangerous",
                "psf/requests",
                "pandas-dev/pandas",
                "simonw/files-to-prompt",
                "simonw/sqlite-utils",
                "pytest-dev/pytest",
                "celery/celery",
                "psf/black",
                "jazzband/pip-tools",
                "python-pillow/Pillow",
                "python-poetry/poetry",
            ]
            response = await analyze(node, repos)
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
