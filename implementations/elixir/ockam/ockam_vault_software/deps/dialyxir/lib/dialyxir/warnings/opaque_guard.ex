defmodule Dialyxir.Warnings.OpaqueGuard do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :opaque_guard
  def warning(), do: :opaque_guard

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([guard | _]) do
    "The guard test #{guard} breaks the opaqueness of its argument."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([guard, args]) do
    "The guard test #{guard}#{args} breaks the opaqueness of its argument."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
