import os
from fastapi import Depends, HTTPException, status, FastAPI
from fastapi.security.api_key import APIKeyHeader
from fastapi import Security
from fastapi.responses import JSONResponse

from ockam import get_logger, NodeDep


def validate_api_key(api_key: str = Security(APIKeyHeader(name="x-api-key", auto_error=False))) -> str:
    expected = os.environ.get("API_KEY")
    if api_key != expected:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid or missing API key header")
    return expected


logger = get_logger("app")
app = FastAPI(dependencies=[Depends(validate_api_key)])


@app.get("/node")
async def get_node(node: NodeDep):
    logger.info(f"the current node is {node.name}...")
    return JSONResponse(content={"node": f"{node.name}"})


@app.post("/analysis")
async def create_analysis(network: str, node: NodeDep):
    logger.info(f"analyzing network {network} with node {node.name}...")
    return JSONResponse(content={"status": f"{network} ok"})
