defmodule Dialyxir.Warnings.UnknownFunction do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :unknown_function
  def warning(), do: :unknown_function

  @impl Dialyxir.Warning
  @spec format_short({String.t(), String.t(), String.t()}) :: String.t()
  def format_short({module, function, arity}) do
    pretty_module = Erlex.pretty_print(module)
    "Function #{pretty_module}.#{function}/#{arity} does not exist."
  end

  @impl Dialyxir.Warning
  @spec format_long({String.t(), String.t(), String.t()}) :: String.t()
  def format_long({module, function, arity}) do
    pretty_module = Erlex.pretty_print(module)
    "Function #{pretty_module}.#{function}/#{arity} does not exist."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
