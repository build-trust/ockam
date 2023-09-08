defmodule Dialyxir.Warnings.RecordConstruction do
  @behaviour Dialyxir.Warning

  @impl Dialyxir.Warning
  @spec warning() :: :record_constr
  def warning(), do: :record_constr

  @impl Dialyxir.Warning
  @spec explain() :: String.t()
  def explain() do
    Dialyxir.Warning.default_explain()
  end

  @impl Dialyxir.Warning
  @spec format_short([String.t()]) :: String.t()
  def format_short([_types, name]) do
    "Record construction violates the declared type for #{name}."
  end

  def format_short([name, _field, _type]) do
    "Record construction violates the declared type for #{name}."
  end

  @impl Dialyxir.Warning
  @spec format_long([String.t()]) :: String.t()
  def format_long([types, name]) do
    "Record construction #{types} violates the declared type for ##{name}{}."
  end

  def format_long([name, field, type]) do
    "Record construction violates the declared type for ##{name}{}, " <>
      "because #{field} cannot be of type #{type}."
  end
end
