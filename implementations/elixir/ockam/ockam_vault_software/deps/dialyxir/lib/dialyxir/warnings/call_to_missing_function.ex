defmodule Dialyxir.Warnings.CallToMissingFunction do
  @moduledoc """
  Function calls a missing or private function. This may be caused by
  a typo or incorrect arity. This is also a compiler warning.

  ## Example

      defmodule Missing do
        def missing(:ok) do
          :ok
        end

        defp missing() do
          :ok
        end
      end

      defmodule Example do
        def error() do
          Missing.missing()
        end
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :call_to_missing
  def warning(), do: :call_to_missing

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short(args), do: format_long(args)

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity]) do
    pretty_module = Erlex.pretty_print(module)
    "Call to missing or private function #{pretty_module}.#{function}/#{arity}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
