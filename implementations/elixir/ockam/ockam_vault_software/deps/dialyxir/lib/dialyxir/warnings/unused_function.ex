defmodule Dialyxir.Warnings.UnusedFunction do
  @moduledoc """
  Due to issues higher in the function or call stack, while the
  function is recognized as used by the compiler, it will never be
  recognized as having been called until the other error is resolved.

  ## Example

      defmodule Example do
        def ok() do
          raise "error"

          unused()
        end

        defp unused(), do: :ok
      end
  """

  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :unused_fun
  def warning(), do: :unused_fun

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short(args), do: format_long(args)

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([function, arity]) do
    "Function #{function}/#{arity} will never be called."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    @moduledoc
  end
end
