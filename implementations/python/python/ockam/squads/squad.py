from dataclasses import dataclass
from typing import Protocol, Optional

from ockam import RemoteNode, WorkerProtocol, LocalNodeProtocol

import json


class SquadWorkerFactoryProtocol(Protocol):
    async def create(self, node: LocalNodeProtocol) -> WorkerProtocol: ...


class Sharding:
    pass


@dataclass
class PerNode(Sharding):
    n: int = 1


@dataclass
class PerItem(Sharding):
    pass


@dataclass
class SquadStep:
    name: str
    worker_factory: SquadWorkerFactoryProtocol
    sharding: Optional[Sharding] = None


def split_list_into_n_parts(lst, n):
    q, r = divmod(len(lst), n)
    return [lst[i * q + min(i, r) : (i + 1) * q + min(i + 1, r)] for i in range(n)]


class SquadWorker:
    node: LocalNodeProtocol
    runners: list[str]
    prev_step_sharding: Optional[Sharding]
    steps: list[SquadStep]

    def __init__(self, runners: list[str], prev_step_sharding: Optional[SquadStep], steps: list[SquadStep]):
        self.runners = runners
        self.prev_step_sharding = prev_step_sharding
        self.steps = steps

    def random_address(self) -> str:
        import secrets

        return secrets.token_hex(6)

    async def handle_message(self, context, message):
        import json
        import ockam

        step = self.steps.pop(0)

        if not self.prev_step_sharding or isinstance(self.prev_step_sharding, PerItem):
            worker_name = self.random_address()
            worker = await step.worker_factory.create(self.node)
            await self.node.start_worker(worker_name, worker)
            reply = await self.node.send_and_receive(worker_name, message, timeout=600)
            await self.node.stop_worker(worker_name)
        elif isinstance(self.prev_step_sharding, PerNode):
            worker_name = self.random_address()
            worker = await step.worker_factory.create(self.node)
            await self.node.start_worker(worker_name, worker)

            items = json.loads(message)
            replies = []
            for item in items:
                reply = await self.node.send_and_receive(worker_name, json.dumps(item), timeout=600)
                reply = json.loads(reply)
                if isinstance(reply, list):
                    replies.extend(reply)
                else:
                    replies.append(reply)

            await self.node.stop_worker(worker_name)
            reply = json.dumps(replies)
        else:
            raise ValueError(f"Invalid sharding: {self.prev_step_sharding}")

        if not step.sharding:
            await context.reply(reply)
        elif isinstance(step.sharding, PerNode):
            # n = step.sharding.n For now support only 1

            async def start(runner: RemoteNode, chunk):
                worker_name = self.random_address()
                worker = SquadWorker(self.runners, step.sharding, self.steps)
                await runner.start_worker(worker_name, worker)

                mailbox_name = self.random_address()
                mailbox = await self.node.create_mailbox(mailbox_name)

                await mailbox.send_to_remote(runner.name, worker_name, json.dumps(chunk))

                return runner, worker_name, mailbox

            async def finish(runner: RemoteNode, worker_name: str, mailbox):
                result = await mailbox.receive(timeout=1000)
                await runner.stop_worker(worker_name)
                return json.loads(result)

            items = json.loads(reply)

            if len(self.runners) > len(items):
                runners = self.runners[: len(items)]
            else:
                runners = self.runners

            chunks = split_list_into_n_parts(items, len(runners))

            futures = [start(RemoteNode(self.node, runner), chunk) for runner, chunk in zip(runners, chunks)]
            handles = await ockam.gather(*futures, batch_size=10)

            futures = [finish(runner, worker_name, mailbox) for runner, worker_name, mailbox in handles]
            results = await ockam.gather(*futures, batch_size=100)

            await context.reply(json.dumps(results))

        elif isinstance(step.sharding, PerItem):

            async def start(item):
                worker_name = self.random_address()
                worker = SquadWorker(self.runners.copy(), step.sharding, self.steps.copy())
                worker.node = self.node
                await self.node.start_worker(worker_name, worker)

                mailbox_name = self.random_address()
                mailbox = await self.node.create_mailbox(mailbox_name)

                await mailbox.send(worker_name, json.dumps(item))

                return worker_name, mailbox

            async def finish(worker_name, mailbox):
                result = await mailbox.receive(timeout=1000)
                await self.node.stop_worker(worker_name)
                return json.loads(result)

            items = json.loads(reply)

            futures = [start(item) for item in items]
            handles = await ockam.gather(*futures, batch_size=10)

            futures = [finish(worker_name, mailbox) for worker_name, mailbox in handles]
            results = await ockam.gather(*futures, batch_size=100)

            await context.reply(json.dumps(results))
        else:
            raise ValueError(f"Invalid sharding: {step.sharding}")


class Squad:
    @staticmethod
    async def run(node: LocalNodeProtocol, runners: list[RemoteNode], data, steps: list[SquadStep]):
        if not steps:
            return []

        runners = list(map(lambda x: x.name, runners))

        worker = SquadWorker(runners, None, steps)
        worker.node = node
        worker_name = worker.random_address()

        await node.start_worker(worker_name, worker)

        result = await node.send_and_receive(worker_name, json.dumps(data), timeout=1000)

        await node.stop_worker(worker_name)

        return result
