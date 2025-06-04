from .remote import RemoteNode
import json


class BoxClient:
    def __init__(self, node):
        self.folders = self

        self.remote_node = RemoteNode(node, "acme-squad-box")

    async def get_folder_items(self, id: str):
        response = await self.remote_node.send_and_receive("box", f"{id}")

        return json.loads(response)

    async def get_text(self, id: str):
        return await self.remote_node.send_and_receive("box", f"text/{id}")
