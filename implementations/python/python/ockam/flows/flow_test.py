import pytest
from .flow import Flow


def test_flow_name():
    # this must not raise an exception
    Flow(name="123-abc")

    # but this should
    with pytest.raises(ValueError):
        Flow(name="!123-abc~")
