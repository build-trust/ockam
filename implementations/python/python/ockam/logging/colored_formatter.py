import colorlog
from datetime import datetime, UTC


class OckamColoredFormatter(colorlog.ColoredFormatter):
    GREY = "\033[90m"
    CYAN = "\033[36m"
    YELLOW = "\033[33m"
    RESET = "\033[0m"

    def __init__(self, *args, **kwargs):
        kwargs.setdefault("datefmt", "%Y-%m-%dT%H:%M:%S.%fZ")
        super().__init__(*args, **kwargs)

    def format(self, record):
        if record.levelname == "WARNING":
            record.levelname = f"{self.YELLOW} WARN{self.RESET}"
        return super().format(record)

    def formatTime(self, record, datefmt=None) -> str:
        try:
            dt = datetime.fromtimestamp(record.created, UTC)
            if datefmt:
                return dt.strftime(datefmt)
            return super().formatTime(record, datefmt)
        except Exception:
            # during shutdown an exception can occur because the time cannot be formatted
            return f"{record.created}"

    def formatMessage(self, record) -> str:
        try:
            record.name = f"{self.GREY}{record.name}{self.RESET}"
            original_asctime = self.formatTime(record, self.datefmt)
            record.asctime = f"{self.GREY}{original_asctime}{self.RESET}"
            return super().formatMessage(record)
        except Exception:
            # during shutdown an exception can occur because the time cannot be formatted
            return super().formatMessage(record)
