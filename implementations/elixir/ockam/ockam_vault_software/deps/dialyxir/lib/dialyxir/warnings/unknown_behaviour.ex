defmodule Dialyxir.Warnings.UnknownBehaviour do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :unknown_behaviour
  def warning(), do: :unknown_behaviour

  @impl Dialyxir.Warning
  @spec format_short(String.t()) :: String.t()
  def format_short(args), do: format_long(args)

  @impl Dialyxir.Warning
  @spec format_long(String.t()) :: String.t()
  def format_long(behaviour) do
    pretty_module = Erlex.pretty_print(behaviour)

    "Unknown behaviour: #{pretty_module}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
