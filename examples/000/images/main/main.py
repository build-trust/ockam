from ockam import Agent, HttpServer, Model, Node

from fastapi import FastAPI, HTTPException, status, Security
from fastapi.security.api_key import APIKeyHeader

from asyncio import gather, create_task
from dataclasses import dataclass
from os import environ
from typing import List, Dict


async def analyze_item(node: Node, item: str) -> Dict[str, str]:
    agent = None
    try:
        agent = await Agent.start(
            node=node,
            instructions="""
                You are an agent that specializes in translating text
                from english to hindi. When you’re given text in english
                output the corresponding translation in hindi written
                using the latin alphabet.
            """,
            model=Model(name="llama4-maverick"),
        )

        response = await agent.send(f"English:\n\n{item}", timeout=1000)
        analysis = response[0].content

        return {"item": item, "analysis": analysis}
    except Exception as e:
        return {"item": item, "error": str(e)}
    finally:
        if agent:
            create_task(Agent.stop(node, agent.name))


async def analyze(node: Node, items: List[str]) -> List[Dict[str, str]]:
    return await gather(*(analyze_item(node, item) for item in items))


class App:
    def __init__(self):
        self.api = FastAPI()

    def routes(self, node: Node):
        app = self.api
        api_key_env = environ["API_KEY"]
        api_key_header = APIKeyHeader(name="x-api-key", auto_error=False)

        def key(api_key_header: str = Security(api_key_header)) -> str:
            if api_key_header == api_key_env:
                return api_key_header
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED,
                detail="Please provide a valid API key.",
            )

        @dataclass
        class GetAnalysesRequest:
            items: List[str]

        @dataclass
        class GetAnalysesResponse:
            analyses: List[Dict[str, str]]

        @app.post("/analyses")
        async def get_analyses(
            request: GetAnalysesRequest, key: str = Security(key)
        ) -> GetAnalysesResponse:
            analyses = await analyze(node, request.items)
            return GetAnalysesResponse(analyses=analyses)


Node.start(http_server=HttpServer(api=App()))
