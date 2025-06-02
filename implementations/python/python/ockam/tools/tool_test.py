import inspect

from .tool import function_spec, parameter_info_from_docstring


def test_function_spec():
    def sample_function(param1: int, param2: str) -> None:
        """A sample function for testing.

        Args:
            param1 (int): An integer parameter.
            param2 (str): A string parameter.
        """
        pass

    name, spec = function_spec(sample_function)
    assert name == "sample_function"

    expected_spec = {
        "type": "function",
        "function": {
            "name": "sample_function",
            "description": inspect.cleandoc("""A sample function for testing.

            Args:
                param1 (int): An integer parameter.
                param2 (str): A string parameter.
            """),
            "parameters": {
                "type": "object",
                "properties": {
                    "param1": {"type": "integer", "description": "parameter param1"},
                    "param2": {"type": "string", "description": "parameter param2"},
                },
                "required": ["param1", "param2"],
            },
        },
    }
    assert spec == expected_spec


def test_function_spec_no_docstring():
    def function_without_docstring(param1: int, param2: str) -> None:
        pass

    name, spec = function_spec(function_without_docstring)
    assert name == "function_without_docstring"

    expected_spec = {
        "type": "function",
        "function": {
            "name": "function_without_docstring",
            "description": "Function function_without_docstring",
            "parameters": {
                "type": "object",
                "properties": {
                    "param1": {"type": "integer", "description": "parameter param1"},
                    "param2": {"type": "string", "description": "parameter param2"},
                },
                "required": ["param1", "param2"],
            },
        },
    }
    assert spec == expected_spec


def test_function_spec_no_type_hints():
    def function_with_docstring_no_type_hints(param1, param2):
        """A sample function for testing.

        Args:
            param1 (int): An integer parameter.
            param2 (str): A string parameter.
        """
        pass

    name, spec = function_spec(function_with_docstring_no_type_hints)
    assert name == "function_with_docstring_no_type_hints"

    expected_spec = {
        "type": "function",
        "function": {
            "name": "function_with_docstring_no_type_hints",
            "description": inspect.cleandoc("""A sample function for testing.

            Args:
                param1 (int): An integer parameter.
                param2 (str): A string parameter.
            """),
            "parameters": {
                "type": "object",
                "properties": {
                    "param1": {"type": "integer", "description": "parameter param1"},
                    "param2": {"type": "string", "description": "parameter param2"},
                },
                "required": ["param1", "param2"],
            },
        },
    }
    assert spec == expected_spec


def test_function_spec_no_docstring_no_type_hints():
    def function_no_docstring_no_type_hints(param1, param2):
        pass

    name, spec = function_spec(function_no_docstring_no_type_hints)
    assert name == "function_no_docstring_no_type_hints"

    expected_spec = {
        "type": "function",
        "function": {
            "name": "function_no_docstring_no_type_hints",
            "description": "Function function_no_docstring_no_type_hints",
            "parameters": {
                "type": "object",
                "properties": {
                    "param1": {"type": "string", "description": "parameter param1"},
                    "param2": {"type": "string", "description": "parameter param2"},
                },
                "required": ["param1", "param2"],
            },
        },
    }
    assert spec == expected_spec


def test_parameter_info_from_docstring():
    docstring = """A sample function for testing.

    Args:
        param1 (int): An integer parameter.
        param2 (str): A string parameter.
    """
    p_info = parameter_info_from_docstring(docstring)

    expected = {"param1": "int", "param2": "str"}
    assert p_info == expected


def test_parameter_info_from_docstring_no_arguments():
    docstring = """A sample function with no parameters."""
    p_info = parameter_info_from_docstring(docstring)

    expected = {}
    assert p_info == expected
