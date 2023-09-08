defmodule Dialyxir.Warnings.OverlappingContract do
  @moduledoc """
  The function has an additional @spec that is already covered more
  generally by a higher @spec.

  ## Example

      defmodule Example do
        @spec ok(atom) :: :ok
        def ok(:ok) do
          :ok
        end

        @spec ok(:error) :: :ok
        def ok(:error) do
          :ok
        end
      end
  """
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :overlapping_contract
  def warning(), do: :overlapping_contract

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function, arity]) do
    "The contract for #{function}/#{arity} is overloaded."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity]) do
    pretty_module = Erlex.pretty_print(module)

    """
    Overloaded contract for #{pretty_module}.#{function}/#{arity} has
    overlapping domains; such contracts are currently unsupported and
    are simply ignored.
    """
  end
end
