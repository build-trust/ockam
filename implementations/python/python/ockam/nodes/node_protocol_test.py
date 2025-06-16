import pytest
from .local import LocalNode
from .remote import RemoteNode


async def test_local_worker_name():
    node = LocalNode(None)
    # this must not raise a ValueError exception
    # the attribute error corresponds to the None worker
    with pytest.raises(AttributeError):
        await node.start_worker(name="123-abc", worker=None)

    # but this should
    with pytest.raises(ValueError):
        await node.start_worker(name="!123-abc~", worker=None)


async def test_remote_worker_name():
    node = RemoteNode(LocalNode(None), "remote")
    # this must not raise a ValueError exception
    # the attribute error corresponds to the None worker
    with pytest.raises(AttributeError):
        await node.start_worker(name="123-abc", worker=None)

    # but this should
    with pytest.raises(ValueError):
        await node.start_worker(name="!123-abc~", worker=None)
