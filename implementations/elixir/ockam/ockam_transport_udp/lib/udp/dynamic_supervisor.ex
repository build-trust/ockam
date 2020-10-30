defmodule Ockam.Transport.UDP.DynamicSupervisor do
  @moduledoc false

  @doc false
  use DynamicSupervisor

  @doc false
  def start_link(_init_arg) do
    DynamicSupervisor.start_link(__MODULE__, nil, name: __MODULE__)
  end

  @impl true
  def init(nil) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end
end
