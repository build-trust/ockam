defmodule Dialyxir.Warnings.OpaqueNonequality do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :opaque_neq
  def warning(), do: :opaque_neq

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([type, _op, opaque_type]) do
    "Attempted to test for inequality between #{type} and opaque type #{opaque_type}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([type, _op, opaque_type]) do
    "Attempt to test for inequality between a term of type #{type} " <>
      "and a term of opaque type #{opaque_type}."
  end

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end
end
