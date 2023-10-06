defmodule Ockam.Examples.Session.Routing.DataWorker do
  @moduledoc false

  use Ockam.AsymmetricWorker

  alias Ockam.Message

  @impl true
  def inner_setup(options, state) do
    route = Keyword.get(options, :route)
    messages = Keyword.get(options, :messages)
    {:ok, Map.merge(state, %{route: route, messages: messages})}
  end

  @impl true
  def handle_inner_message(message, state) do
    Ockam.Worker.route(Message.forward(message), state)

    {:ok, Map.update(state, :messages, [message], fn messages -> [message | messages] end)}
  end

  @impl true
  def handle_outer_message(message, state) do
    [_ | onward_route] = Message.onward_route(message)
    ## TODO: add forward_through?
    Ockam.Worker.route(Message.set_onward_route(message, state.route ++ onward_route), state)

    {:ok, Map.update(state, :messages, [message], fn messages -> [message | messages] end)}
  end
end
