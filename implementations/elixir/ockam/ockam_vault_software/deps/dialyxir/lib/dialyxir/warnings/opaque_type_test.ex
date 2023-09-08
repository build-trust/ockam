defmodule Dialyxir.Warnings.OpaqueTypeTest do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :opaque_type_test
  def warning(), do: :opaque_type_test

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([function, _opaque]) do
    "The type test in #{function} breaks the opaqueness of the term."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([function, opaque]) do
    "The type test #{function}(#{opaque}) breaks the opaqueness of the term #{opaque}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
