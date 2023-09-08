defmodule Dialyxir.Warnings.UnknownType do
  @moduledoc """
  Spec references a missing @type.

  ## Example

      defmodule Missing do
      end

      defmodule Example do
        @spec ok(Missing.t()) :: :ok
        def ok(_) do
          :ok
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :unknown_type
  def warning(), do: :unknown_type

  @impl Dialyxir.Warning
  @spec format_short({String.t(), String.t(), String.t()}) :: String.t()
  def format_short({module, function, arity}) do
    pretty_module = Erlex.pretty_print(module)

    "Unknown type: #{pretty_module}.#{function}/#{arity}."
  end

  @impl Dialyxir.Warning
  @spec format_long({String.t(), String.t(), String.t()}) :: String.t()
  def format_long({module, function, arity}) do
    format_short({module, function, arity})
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
