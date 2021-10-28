defmodule Ockam.Messaging.Delivery.Tests.Filter do
  @moduledoc false

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
