from contextlib import contextmanager

import os
import logging.config
from typing import Protocol

DEFAULT_LOG_FORMAT = os.getenv(
    "DEFAULT_LOG_FORMAT", "%(asctime)s %(log_color)s%(levelname)5s%(reset)s %(name)-14s %(message)s"
)
FORMAT = DEFAULT_LOG_FORMAT + (" [%(pathname)s:%(lineno)d]" if os.getenv("OCKAM_LOG_SHOW_SOURCE", False) else "")

LOG_LEVELS = {}

LEVELS: dict[str, int] = {
    "critical": logging.CRITICAL,
    "error": logging.ERROR,
    "warning": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}


def get_logging_config() -> dict[str, int | bool | dict | str | None]:
    # Disable logging if explicitly set to 0; otherwise, assume it's enabled
    if os.environ.get("OCKAM_LOGGING", "1") == "0":
        return {
            "version": 1,
        }

    global LOG_LEVELS
    if not LOG_LEVELS:
        set_log_levels(None)
    return create_logging_config(LOG_LEVELS, FORMAT)


def set_log_level(module_name: str, level: str):
    """
    Set the log level for a specific module.
    """
    global LOG_LEVELS
    if not LOG_LEVELS:
        LOG_LEVELS = create_log_levels(None)
    LOG_LEVELS[module_name] = level.upper()


def set_log_levels(log_levels: str):
    global LOG_LEVELS
    if not LOG_LEVELS:
        LOG_LEVELS = create_log_levels(None)
    LOG_LEVELS = create_log_levels(log_levels)

    # By default, silence the Ockam rust modules
    if os.environ.get("OCKAM_LOG_LEVEL") is None or LOG_LEVELS.get("ockam_rust_modules") is not None:
        os.environ["OCKAM_LOG_LEVEL"] = LOG_LEVELS.get("ockam_rust_modules")


def create_logging_config(levels: dict, log_format: str) -> dict[str, int | bool | dict | str | None]:
    return {
        "version": 1,
        "disable_existing_loggers": False,
        "formatters": {
            "default": {
                "()": "ockam.logging.colored_formatter.OckamColoredFormatter",
                "format": log_format,
                "log_colors": {
                    "DEBUG": "blue",
                    "INFO": "green",
                    "WARNING": "yellow",
                    "ERROR": "red",
                    "CRITICAL": "bold_red",
                },
            },
        },
        "handlers": {
            "default": {
                "level": levels.get("default"),
                "formatter": "default",
                "class": "logging.StreamHandler",
            },
            "ockam": {
                "level": levels.get("ockam"),
                "formatter": "default",
                "class": "logging.StreamHandler",
            },
        },
        "loggers": {
            "asyncio": {"handlers": ["default"], "level": levels.get("asyncio", "WARNING"), "propagate": False},
            "httpcore": {"handlers": ["default"], "level": levels.get("httpcore", "WARNING"), "propagate": False},
            "httpx": {"handlers": ["default"], "level": levels.get("httpx", "WARNING"), "propagate": False},
            "LiteLLM": {"handlers": ["default"], "level": levels.get("LiteLLM", "WARNING"), "propagate": False},
            "LiteLLM Router": {
                "handlers": ["default"],
                "level": levels.get("LiteLLM Router", "WARNING"),
                "propagate": False,
            },
            "LiteLLM Proxy": {
                "handlers": ["default"],
                "level": levels.get("LiteLLM Proxy", "WARNING"),
                "propagate": False,
            },
            "uvicorn": {"handlers": ["default"], "level": levels.get("uvicorn", "WARNING"), "propagate": False},
            "uvicorn.access": {
                "handlers": ["default"],
                "level": levels.get("uvicorn.access", "WARNING"),
                "propagate": False,
            },
            "uvicorn.asgi": {
                "handlers": ["default"],
                "level": levels.get("uvicorn.asgi", "WARNING"),
                "propagate": False,
            },
            "uvicorn.error": {
                "handlers": ["default"],
                "level": levels.get("uvicorn.error", "WARNING"),
                "propagate": False,
            },
            "agent": {
                "handlers": ["ockam"],
                "level": levels.get("agent") or levels.get("default"),
                "propagate": False,
            },
            "http": {"handlers": ["ockam"], "level": levels.get("http") or levels.get("default"), "propagate": False},
            "knowledge": {
                "handlers": ["ockam"],
                "level": levels.get("knowledge") or levels.get("default"),
                "propagate": False,
            },
            "mem0.memory.main": {
                "handlers": ["default"],
                "level": levels.get("mem0.memory.main") or levels.get("default"),
                "propagate": False,
            },
            "memory": {
                "handlers": ["ockam"],
                "level": levels.get("memory") or levels.get("default"),
                "propagate": False,
            },
            "model": {
                "handlers": ["default"],
                "level": levels.get("model") or levels.get("default"),
                "propagate": False,
            },
            "node": {"handlers": ["ockam"], "level": levels.get("node") or levels.get("default"), "propagate": False},
            "searchable_knowledge": {
                "handlers": ["ockam"],
                "level": levels.get("searchable_knowledge") or levels.get("default"),
                "propagate": False,
            },
            "squad": {"handlers": ["ockam"], "level": levels.get("squad") or levels.get("default"), "propagate": False},
            "tool": {"handlers": ["ockam"], "level": levels.get("tool") or levels.get("default"), "propagate": False},
            "user": {"handlers": ["ockam"], "level": levels.get("user") or levels.get("default"), "propagate": False},
        },
        "root": {"level": levels.get("default"), "handlers": ["default"]},
    }


def create_log_levels(log_levels: str) -> dict[str, str]:
    """
    Create log levels for python modules
    """
    result = {"default": "INFO", "ockam_rust_modules": "WARN"}
    if log_levels is not None:
        # set log levels for each python module if defined in the log_levels string
        ockam_rust_levels = []
        for level in log_levels.split(","):
            key_value = level.split("=")
            if len(key_value) == 1:
                result["default"] = level.upper()
            else:
                key = key_value[0].strip()
                value = key_value[1].strip()
                if key.startswith("ockam_") or "ockam" == key:
                    ockam_rust_levels.append(f"{key}={value}")
                else:
                    result[key] = value.upper()
        # if the user does not specify anything for the ockam rust modules, then set them to warn
        if len(ockam_rust_levels) > 0:
            result["ockam_rust_modules"] = ",".join(ockam_rust_levels)

    return result


def get_logger(logger_name):
    logging.config.dictConfig(get_logging_config())
    return logging.getLogger(logger_name)


def info(msg, *args, **kwargs):
    logger = logging.getLogger("user")
    logger.info(msg, stacklevel=2, *args, **kwargs)


def warning(msg, *args, **kwargs):
    logger = logging.getLogger("user")
    logger.warning(msg, stacklevel=2, *args, **kwargs)


def debug(msg, *args, **kwargs):
    logger = logging.getLogger("user")
    logger.debug(msg, stacklevel=2, *args, **kwargs)


def error(msg, *args, **kwargs):
    logger = logging.getLogger("user")
    logger.error(msg, stacklevel=2, *args, **kwargs)


class LoggerAware(Protocol):
    logger: logging.Logger


class InfoContext(LoggerAware):
    @contextmanager
    def info(self, before_msg, after_msg):
        self.logger.info(before_msg)
        try:
            yield
        except Exception as e:
            self.logger.info(f"{e}")
            return
        self.logger.info(after_msg)


class DebugContext(LoggerAware):
    @contextmanager
    def debug(self, before_msg, after_msg):
        self.logger.debug(before_msg)
        try:
            yield
        except Exception as e:
            self.logger.debug(f"{e}")
            return
        self.logger.debug(after_msg)
