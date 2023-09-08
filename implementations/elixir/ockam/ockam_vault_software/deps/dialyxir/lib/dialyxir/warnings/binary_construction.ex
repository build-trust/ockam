defmodule Dialyxir.Warnings.BinaryConstruction do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :bin_construction
  def warning(), do: :bin_construction

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([culprit | _]) do
    "Binary construction with #{culprit} will fail."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([culprit, size, segment, type]) do
    pretty_type = Erlex.pretty_print_type(type)

    "Binary construction will fail since the #{culprit} field #{size} in " <>
      "segment #{segment} has type #{pretty_type}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
