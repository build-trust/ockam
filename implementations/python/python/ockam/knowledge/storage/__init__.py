from .database import Database
from .in_memory import InMemory
from .protocol import Storage

import os

storage = None


def create_storage() -> Storage:
    """
    Factory function to create a storage instance.
    """

    # use the database storage when available
    # make it a global singleton so that it is reused across the application
    if os.environ.get("OCKAM_DATABASE_INSTANCE"):
        global storage
        if not storage:
            storage = Database()
        return storage

    return InMemory()
