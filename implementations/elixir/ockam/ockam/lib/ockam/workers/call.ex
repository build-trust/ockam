defmodule Ockam.Workers.Call do
  @moduledoc """
  One-off worker to perform a synchronous call to an Ockam worker using
  the routing protocol.
  """

  use Ockam.Worker

  alias Ockam.Message

  require Logger

  def call(call, options \\ [], timeout \\ 10_000) do
    {:ok, address} = __MODULE__.create(Keyword.put(options, :call, call))
    GenServer.call(Ockam.Node.whereis(address), :fetch, timeout)
  end

  @impl true
  def setup(options, state) do
    call = Keyword.fetch!(options, :call)
    send_call(call, state)
    {:ok, state}
  end

  def send_call(call, state) do
    Ockam.Router.route(%{
      payload: Message.payload(call),
      onward_route: Message.onward_route(call),
      return_route: [state.address]
    })
  end

  @impl true
  def handle_message(message, state) do
    Map.put(state, :message, message)

    case Map.get(state, :wait) do
      nil ->
        {:ok, state}

      waiter ->
        GenServer.reply(waiter, message)
        ## Terminate here
        {:stop, :shutdown, state}
    end
  end

  @impl true
  def handle_call(:fetch, from, state) do
    case Map.get(state, :message) do
      nil ->
        {:noreply, Map.put(state, :wait, from)}

      message ->
        GenServer.reply(from, message)
        ## Terminate here
        {:stop, :shutdown, state}
    end
  end
end
