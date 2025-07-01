from fastapi import FastAPI
from fastapi.responses import JSONResponse

from ockam import info


class Api:
    def __init__(self):
        self.api = FastAPI()

    def routes(self, node):
        @self.api.post("/analysis")
        async def create_analysis(network: str):
            info(f"Analyzing network {network} with node {node.name}...")
            return JSONResponse(content={"status": "ok"})
