import re


def validate_name(name: str):
    """
    Raises ValueError if `name` contains characters other than a-z, A-Z, 0-9, -, or _.
    """
    if not re.fullmatch(r"[a-zA-Z0-9_-]+", name):
        raise ValueError(f"Invalid name '{name}'. Only alphanumeric characters, '-' and '_' are allowed.")
