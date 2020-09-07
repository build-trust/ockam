defmodule Ockam.Nodes do
  @moduledoc """
  Ockam.Nodes
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

defmodule Ockam.Node do
  @moduledoc """
  Ockam Node
  """

  use GenServer

  alias Ockam.Router

  require Logger

  # Starts controller process linked to the current process
  @doc false
  def start_link(_options) do
    GenServer.start_link(__MODULE__, nil, name: {:via, Router, {0, 0}})
  end

  @doc false
  @impl true
  def init(_options), do: {:ok, []}

  @doc false
  @impl true
  def handle_info(_message, state) do
    {:noreply, state}
  end
end
