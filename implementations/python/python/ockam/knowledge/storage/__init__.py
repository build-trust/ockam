from .database import Database
from .in_memory import InMemory
from .protocol import Storage

def create_storage() -> Storage:
    """
    Factory function to create a storage instance.
    """

    # prefer the database storage when available
    storage = Database.from_environment(
        return_none_if_env_missing=True
    )
    if storage:
        return storage

    return InMemory()
