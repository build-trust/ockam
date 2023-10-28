defmodule Ockam.Examples.Messaging.Shuffle do
  @moduledoc """
  Worker to shuffle forwarded messages

  Spawns a process for each message
  """
  use Ockam.Worker

  alias Ockam.Message

  require Logger

  @impl true
  def handle_message(message, state) do
    spawn(fn ->
      forward_message(message, state)
    end)

    {:ok, state}
  end

  def forward_message(message, state) do
    :timer.sleep(10)
    Ockam.Worker.route(Message.forward(message), state)
  end
end
