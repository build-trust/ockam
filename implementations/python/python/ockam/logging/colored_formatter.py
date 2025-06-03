import colorlog

class OckamColoredFormatter(colorlog.ColoredFormatter):
    GREY = "\033[90m"
    CYAN = "\033[36m"
    RESET = "\033[0m"

    def formatMessage(self, record):
        original_asctime = self.formatTime(record, self.datefmt)
        record.asctime = f"{self.CYAN}{original_asctime}{self.RESET}"
        record.name = f"{self.GREY}{record.name}{self.RESET}"
        return super().formatMessage(record)
