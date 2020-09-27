defmodule Ockam.Topics do
  @moduledoc """
  Ockam.Topics
  """

  @doc false
  use DynamicSupervisor

  require Logger

  def start_link(init_arg) do
    DynamicSupervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end
end

defmodule Ockam.Topic do
  @moduledoc """
  Implements the publish and subscribe semantics.
  """
end
