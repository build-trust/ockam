defmodule Ockam.Examples.Session.CountTo.Handshake do
  @moduledoc """
  Example multi-message handshake

  Counts to a `count_to` number, each new handshake message
  increasing the number, starting the data worker when reaching `count_to`

  Options:

  `count_to` - number of handshakes to send
  """
  @behaviour Ockam.Session.Handshake

  alias Ockam.Message

  def init(options, state) do
    count_to = Keyword.fetch!(options, :count_to)

    state = Map.merge(state, %{count_to: count_to, count: 0})

    {:next, count_message(state, count_to, 0, state.init_route), state}
  end

  def handle_initiator(options, message, state) do
    do_count(options, message, state)
  end

  def handle_responder(options, message, state) do
    do_count(options, message, state)
  end

  def do_count(options, message, state) do
    return_route = Message.return_route(message)
    payload = Message.payload(message)
    count = String.to_integer(payload)

    count_to = Keyword.fetch!(options, :count_to)

    next = count + 1

    case next do
      less when less < count_to ->
        {:next, count_message(state, count_to, next, return_route), Map.put(state, :count, next)}

      eq when eq == count_to ->
        {:ready, count_message(state, count_to, next, return_route), [count: next],
         Map.put(state, :count, next)}

      more when more > count_to ->
        {:ready, [count: count], Map.put(state, :count, count)}
    end
  end

  def count_message(state, count_to, count, route) when count <= count_to do
    %{
      onward_route: route,
      return_route: [state.handshake_address],
      payload: "#{count}"
    }
  end
end
