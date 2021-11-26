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
    Ockam.Router.route(Message.forward_trace(message))
  end
end
