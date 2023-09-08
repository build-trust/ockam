defmodule Dialyxir.Warnings.AppCall do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :app_call
  def warning(), do: :app_call

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_module, function | _]) do
    "Module or function to apply is not an atom in #{function}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([module, function, arity, culprit, expected_type, actual_type]) do
    pretty_module = Erlex.pretty_print(module)
    pretty_expected_type = Erlex.pretty_print_type(expected_type)
    pretty_actual_type = Erlex.pretty_print_type(actual_type)

    "The call #{pretty_module}.#{function}/#{arity} requires that " <>
      "#{culprit} is of type #{pretty_expected_type}, not #{pretty_actual_type}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
