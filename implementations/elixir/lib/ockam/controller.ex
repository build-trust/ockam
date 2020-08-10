defmodule Ockam.Controller do
  @moduledoc """
  Defines an Ockam Controller
  """

  use GenServer

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @doc false
  def start_link(_options) do
    GenServer.start_link(__MODULE__, nil, name: {:via, Router, 0})
  end

  @doc false
  @impl true
  def init(_options), do: {:ok, []}

  @doc false
  @impl true
  def handle_info(%Message{payload: :ping, return_route: return_route} = incoming, state) do
    Logger.debug("Controller: #{inspect({incoming, state})}")
    Router.route(%Message{payload: :pong, onward_route: return_route})
    {:noreply, state}
  end

  # def handle_info(%Message{payload: {:create_channel_xx_message_1, _}}, state) do
  #   {:noreply, state}
  # end

  def handle_info(_message, state), do: {:noreply, state}
end
