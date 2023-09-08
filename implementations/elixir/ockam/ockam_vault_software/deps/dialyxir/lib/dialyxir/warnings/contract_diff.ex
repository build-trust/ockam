defmodule Dialyxir.Warnings.ContractDiff do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :contract_diff
  def warning(), do: :contract_diff

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "Type specification is not equal to the success typing for #{function}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity, contract, signature]) do
    pretty_module = Erlex.pretty_print(module)
    pretty_contract = Erlex.pretty_print_contract(contract)
    pretty_signature = Erlex.pretty_print_contract(signature)

    """
    Type specification is not equal to the success typing.

    Function:
    #{pretty_module}.#{function}/#{arity}

    Type specification:
    #{pretty_contract}

    Success typing:
    #{pretty_signature}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
