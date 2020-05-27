defmodule Ockam.Registry do
  @moduledoc """
  Implements the Ockam Registry.
  """

  use Agent

  def start_link(_opts) do
    Agent.start_link(fn -> %{} end)
  end

  def get(registry, key) do
    Agent.get(registry, &Map.get(&1, key))
  end

  def put(registry, key, value) do
    Agent.update(registry, &Map.put(&1, key, value))
  end
end
