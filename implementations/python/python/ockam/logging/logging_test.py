import os
import pytest

from .logging import create_log_levels


@pytest.fixture(autouse=True)
def clear_ockam_log_level():
    os.environ.pop("OCKAM_LOG_LEVEL", None)
    yield
    os.environ.pop("OCKAM_LOG_LEVEL", None)


def test_create_log_levels_when_no_variable_is_defined():
    result = create_log_levels(None)
    assert result == {"default": "INFO", "ockam_rust_modules": "WARN"}, (
        "the default level for python modules is info, rust modules is warn"
    )


def test_create_log_levels_when_only_a_level_is_defined():
    result = create_log_levels("DEBUG")

    assert result == {"default": "DEBUG", "ockam_rust_modules": "WARN"}, (
        "the default level for python modules is debug, rust modules is warn"
    )


def test_create_log_levels_when_only_a_module_is_defined():
    result = create_log_levels("agent=debug")

    assert result == {"default": "INFO", "agent": "DEBUG", "ockam_rust_modules": "WARN"}, (
        "the default level for python modules is info, debug for the agent module, rust modules is warn"
    )


def test_create_log_levels_when_some_python_modules_are_defined():
    result = create_log_levels("DEBUG,agent=info")

    assert result == {"default": "DEBUG", "agent": "INFO", "ockam_rust_modules": "WARN"}, (
        "the default level for python modules is debug, agent is info, rust modules is warn"
    )


def test_create_log_levels_when_some_python_modules_and_ockam_modules_are_defined():
    result = create_log_levels("DEBUG,agent=info,ockam_core=info")

    assert result == {"default": "DEBUG", "agent": "INFO", "ockam_rust_modules": "ockam_core=info"}, (
        "the default level for python modules is debug, agent is info, rust modules is info for ockam_core only"
    )
    assert result.get("node", result.get("default")) == "DEBUG", (
        "the default level can be used for an unspecified module"
    )
