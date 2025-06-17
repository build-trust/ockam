import json
import inspect
import re

from typing import get_type_hints, Optional
from functools import wraps
from docstring_parser import parse

from .protocol import InvokableTool


class Tool(InvokableTool):
    def __init__(self, func):
        name, spec = function_spec(func)
        self.func = wrap(func)
        self.name = name
        self._spec = spec
        self.node = None
        if re.match(r"^[a-z0-9_-]+$", spec["function"]["name"]) is None:
            raise ValueError("Tool name may only contain [a-z0-9_-] characters")

    async def spec(self) -> dict:
        return self._spec

    async def invoke(self, json_argument: Optional[str]) -> str:
        if json_argument is None or json_argument == "":
            args = {}
        else:
            args = json.loads(json_argument)
        tool_response = str(await self.func(**args))
        return tool_response


def wrap(f) -> callable:
    @wraps(f)
    async def wrapper(**kwargs):
        r = f(**kwargs)
        if inspect.iscoroutine(r):
            return await r
        return r

    return wrapper


def function_spec(f) -> (str, dict):
    f_name = f.__name__
    f_description = inspect.cleandoc(f.__doc__) if f.__doc__ else f"Function {f_name}"
    f_parameters = parameters_spec(f)
    return f_name, {
        "type": "function",
        "function": {"name": f_name, "description": f_description, "parameters": f_parameters},
    }


def parameters_spec(f):
    f_parameters = {"type": "object", "properties": {}, "required": []}

    signature = inspect.signature(f)
    type_hints = get_type_hints(f)

    p_info_from_docstring = parameter_info_from_docstring(f.__doc__)
    for p_name, p in signature.parameters.items():
        # Get parameter type from typehint and docstring
        # Prefer the value from the docstring if available.
        # Convert from python type to json schema type
        p_type = str(type_hints.get(p_name, "Any")).replace("<class '", "").replace("'>", "")
        p_type = p_info_from_docstring.get(p_name, p_type)
        p_type = to_json_schema_type(p_type)

        f_parameters["properties"][p_name] = {"type": p_type, "description": f"parameter {p_name}"}
        f_parameters["required"].append(p_name)

    return f_parameters


def to_json_schema_type(p_type):
    return {
        "bool": "boolean",
        "int": "integer",
        "float": "number",
        "str": "string",
        "list": "array",
        "dict": "object",
    }.get(p_type, "string")


def parameter_info_from_docstring(docstring):
    p_info = {}
    if not docstring:
        return p_info

    parsed = parse(docstring)
    for parameter in parsed.params:
        p_name = parameter.arg_name
        p_type = parameter.type_name
        p_info[p_name] = p_type

    return p_info
