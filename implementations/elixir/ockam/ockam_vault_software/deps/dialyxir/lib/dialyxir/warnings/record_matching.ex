defmodule Dialyxir.Warnings.RecordMatching do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :record_matching
  def warning(), do: :record_matching

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short(args), do: format_long(args)

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([string, name]) do
    "The #{string} violates the declared type for ##{name}{}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
