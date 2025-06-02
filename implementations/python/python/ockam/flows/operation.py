from enum import Enum

START = "__**START**__"
END = "__**END**__"


class FlowOperation(Enum):
    ROUTE = "route"
    EVALUATE = "evaluate"
    TRY_AGAIN = "try_again"
