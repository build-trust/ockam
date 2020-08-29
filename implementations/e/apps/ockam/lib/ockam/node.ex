defmodule Ockam.Node do
  @moduledoc """
  Ockam Node
  """

  use GenServer

  alias Ockam.Routing
  alias Ockam.Routing.Address

  require Logger

  # Starts controller process linked to the current process
  @doc false
  def start_link(_options) do
    GenServer.start_link(__MODULE__, nil, name: {:via, Routing, Routing.default_address()})
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
