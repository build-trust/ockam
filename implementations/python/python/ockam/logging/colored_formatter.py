import colorlog
from datetime import datetime, UTC


class OckamColoredFormatter(colorlog.ColoredFormatter):
    GREY = "\033[90m"
    CYAN = "\033[36m"
    RESET = "\033[0m"

    def __init__(self, *args, **kwargs):
        kwargs.setdefault("datefmt", "%Y-%m-%dT%H:%M:%S.%fZ")
        super().__init__(*args, **kwargs)

    def formatTime(self, record, datefmt=None):
        try:
            dt = datetime.fromtimestamp(record.created, UTC)
            if datefmt:
                return dt.strftime(datefmt)
            return super().formatTime(record, datefmt)
        except Exception:
            return ""

    def formatMessage(self, record):
        try:
            original_asctime = self.formatTime(record, self.datefmt)
            record.asctime = f"{self.GREY}{original_asctime}{self.RESET}"
            record.name = f"{self.GREY}{record.name}{self.RESET}"
            return super().formatMessage(record)
        except Exception:
            return ""
