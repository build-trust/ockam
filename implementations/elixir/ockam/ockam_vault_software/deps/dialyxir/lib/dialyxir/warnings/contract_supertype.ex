defmodule Dialyxir.Warnings.ContractSupertype do
  @moduledoc """
  The @spec, while not incorrect, is more general than the type
  returned by the function.

  ## Example

      defmodule Example do
        @spec ok() :: any
        def ok() do
          :ok
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :contract_supertype
  def warning(), do: :contract_supertype

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "Type specification for #{function} is a supertype of the success typing."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity, contract, signature]) do
    pretty_module = Erlex.pretty_print(module)
    pretty_contract = Erlex.pretty_print_contract(contract)
    pretty_signature = Erlex.pretty_print_contract(signature)

    """
    Type specification is a supertype of the success typing.

    Function:
    #{pretty_module}.#{function}/#{arity}

    Type specification:
    @spec #{function}#{pretty_contract}

    Success typing:
    @spec #{function}#{pretty_signature}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
