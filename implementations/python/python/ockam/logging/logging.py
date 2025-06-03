from ..logging.colored_formatter import OckamColoredFormatter

LOGGING_CONFIG = {
    "version": 1,
    "disable_existing_loggers": False,
    "formatters": {
        "default": {
            "()": "ockam.logging.colored_formatter.OckamColoredFormatter",
            "format": "[%(asctime)s] %(log_color)s[%(levelname)s]%(reset)s %(name)s: %(message)s",
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
            "level": "INFO",
            "formatter": "default",
            "class": "logging.StreamHandler",
        },
    },
    "loggers": {
        "uvicorn": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "uvicorn.error": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "uvicorn.access": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "node": {"handlers": ["default"], "level": "INFO", "propagate": False},
        "http": {"handlers": ["default"], "level": "INFO", "propagate": False},
    },
    "root": {"level": "INFO", "handlers": ["default"]},
}

