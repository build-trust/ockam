defmodule Dialyxir.Warnings.ContractWithOpaque do
  @moduledoc """
  The @spec says the function is returning an opaque type, but it is
  returning a different type.

  ## Example

      defmodule Types do
        @opaque type :: :ok
      end

      defmodule Example do
        @spec ok() :: Types.type()
        def ok() do
          :ok
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :contract_with_opaque
  def warning(), do: :contract_with_opaque

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "The @spec for #{function} has an opaque subtype which is violated by the success typing."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity, type, signature_type]) do
    pretty_module = Erlex.pretty_print(module)
    pretty_type = Erlex.pretty_print_type(type)
    pretty_success_type = Erlex.pretty_print_contract(signature_type)

    """
    The @spec for #{pretty_module}.#{function}/#{arity} has an opaque
    subtype #{pretty_type} which is violated by the success typing.

    Success typing:
    #{pretty_success_type}
    """
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
