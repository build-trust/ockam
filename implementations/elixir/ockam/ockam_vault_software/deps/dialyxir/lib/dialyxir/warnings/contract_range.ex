defmodule Dialyxir.Warnings.ContractRange do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :contract_range
  def warning(), do: :contract_range

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_, _, function | _]) do
    "Contract cannot be correct because return type for #{function} is mismatched."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([contract, module, function, arg_strings, line, contract_return]) do
    pretty_contract = Erlex.pretty_print_contract(contract)
    pretty_module = Erlex.pretty_print(module)
    pretty_contract_return = Erlex.pretty_print_type(contract_return)
    pretty_args = Erlex.pretty_print_args(arg_strings)

    """
    Contract cannot be correct because return type on line number #{line} is mismatched.

    Function:
    #{pretty_module}.#{function}#{pretty_args}

    Type specification:
    #{pretty_contract}

    Success typing (line #{line}):
    #{pretty_contract_return}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
