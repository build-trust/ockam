defmodule Ockam.Examples.Messaging.Filter do
  @moduledoc """
  Filter worker

  Randomly drops 5% of messages, forwards the rest to onward_route
  Adds itself to return_route
  """

  use Ockam.Worker

  alias Ockam.Message

  @impl true
  def handle_message(message, state) do
    if :rand.uniform(100) > 5 do
      forward_message(message)
    end

    {:ok, state}
  end

  def forward_message(message) do
    [me | onward_route] = Message.onward_route(message)

    Ockam.Router.route(%{
      onward_route: onward_route,
      return_route: [me | Message.return_route(message)],
      payload: Message.payload(message)
    })
  end
end
