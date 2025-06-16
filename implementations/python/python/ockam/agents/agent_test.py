import pytest

from .agent import Agent


async def test_agent_name():
    with pytest.raises(ValueError):
        await Agent.start(name="!123-abc~", exposed_as="henry", node=None, instructions=None)
