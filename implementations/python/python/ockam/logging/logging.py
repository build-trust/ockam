import logging.config
import os

LOGGING_CONFIG = {
    "version": 1,
    "disable_existing_loggers": False,
    "formatters": {
        "default": {
            "()": "ockam.logging.colored_formatter.OckamColoredFormatter",
            "format": "%(asctime)s %(log_color)s%(levelname)s%(reset)s %(name)s: %(message)s",
            "log_colors": {
                "DEBUG": "cyan",
                "INFO": "green",
                "WARNING": "yellow",
                "ERROR": "red",
                "CRITICAL": "bold_red",
            },
        },
    },
    "handlers": {
        "default": {
            "level": "DEBUG",
            "formatter": "default",
            "class": "logging.StreamHandler",
        },
    },
    "loggers": {
        "uvicorn": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "uvicorn.error": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "uvicorn.access": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "http": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "httpx": {"handlers": ["default"], "level": "WARNING", "propagate": False},
        "LiteLLM": {"handlers": ["default"], "level": "WARNING", "propagate": False},
        "agent": {"handlers": ["default"], "level": "DEBUG", "propagate": False},
        "node": {"handlers": ["default"], "level": "DEBUG", "propagate": False},
    },
    "root": {"level": "INFO", "handlers": ["default"]},
}


def get_logging_config():
    config = LOGGING_CONFIG
    # Disable logging if explicitly set to 0; otherwise, assume it's enabled
    if os.environ.get("OCKAM_LOGGING", "1") == "0":
        return {
            "version": 1,
        }
    return config


def info(msg, *args, **kwargs):
    logger = logging.getLogger("node")
    logger.info(msg, *args, **kwargs)


def warning(msg, *args, **kwargs):
    import logging.config

    logger = logging.getLogger("node")
    logger.warning(msg, *args, **kwargs)


def debug(msg, *args, **kwargs):
    import logging.config

    logger = logging.getLogger("node")
    logger.debug(msg, *args, **kwargs)


def error(msg, *args, **kwargs):
    import logging.config

    logger = logging.getLogger("node")
    logger.error(msg, *args, **kwargs)


def set_log_level(logger_name, level):
    logging_config = get_logging_config()
    if logger_name in logging_config.get("loggers", {}):
        logging_config["loggers"][logger_name]["level"] = level
        import logging.config

        logging.config.dictConfig(logging_config)
