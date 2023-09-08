defmodule Dialyxir.Warnings.ContractSubtype do
  # TODO: could not create warning with this example (and --overspecs)
  @moduledoc """
  The type in the @spec does not completely cover the types returned
  by function.

  ## Example

      defmodule Example do
        @spec ok(:ok | :error) :: :ok
        def ok(:ok) do
          :ok
        end

        def ok(:error) do
          :error
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :contract_subtype
  def warning(), do: :contract_subtype

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "Type specification for #{function} is a subtype of the success typing."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity, contract, signature]) do
    pretty_module = Erlex.pretty_print(module)
    pretty_signature = Erlex.pretty_print_contract(signature)
    pretty_contract = Erlex.pretty_print_contract(contract, module, function)

    """
    Type specification is a subtype of the success typing.

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
