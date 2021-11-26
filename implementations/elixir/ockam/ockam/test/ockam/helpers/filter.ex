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
    Ockam.Router.route(Message.forward_trace(message))
  end
end
