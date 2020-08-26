defmodule Ockam.Control do
  @moduledoc """
  Implements the Ockam Control Protocol.
  """

  use GenServer

  alias Ockam.Routing
  alias Ockam.Routing.Address

  require Logger

  @doc """
  Returns the address of the controller process.
  """
  @spec controller_address() :: Address.t()
  def controller_address, do: Routing.default_address()

  # Starts controller process linked to the current process
  @doc false
  def start_link(_options) do
    GenServer.start_link(__MODULE__, nil, name: {:via, Routing, controller_address()})
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
