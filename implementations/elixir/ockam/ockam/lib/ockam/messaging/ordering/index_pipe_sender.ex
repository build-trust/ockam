defmodule Ockam.Messaging.Ordering.IndexPipe.Sender do
  @moduledoc """
  Sender side of ordered pipe using indexing to enforce ordering
  Each incoming message is assigned an monotonic index, wrapped and sent to receiver

  Options:

  `receiver_route` - a route to receiver
  """

  use Ockam.Worker

  alias Ockam.Message

  alias Ockam.Messaging.Ordering.IndexPipe.Wrapper

  @impl true
  def setup(options, state) do
    receiver_route = Keyword.fetch!(options, :receiver_route)
    {:ok, Map.put(state, :receiver_route, receiver_route)}
  end

  @impl true
  def handle_message(message, state) do
    {indexed_message, state} = make_indexed_message(message, state)
    Ockam.Router.route(indexed_message)
    {:ok, state}
  end

  defp make_indexed_message(message, state) do
    {next_index, state} = next_index(state)
    [_ | onward_route] = Message.onward_route(message)

    forwarded_message = %{
      onward_route: onward_route,
      return_route: Message.return_route(message),
      payload: Message.payload(message)
    }

    indexed_message = %{
      onward_route: receiver_route(state),
      return_route: local_address(state),
      payload: Wrapper.wrap_message(next_index, forwarded_message)
    }

    {indexed_message, state}
  end

  defp next_index(state) do
    index = Map.get(state, :last_index, 0) + 1
    {index, Map.put(state, :last_index, index)}
  end

  defp receiver_route(state) do
    Map.get(state, :receiver_route)
  end

  defp local_address(state) do
    Map.get(state, :address)
  end
end
