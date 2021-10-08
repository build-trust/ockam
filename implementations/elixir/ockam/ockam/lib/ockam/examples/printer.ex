defmodule Ockam.Examples.Printer do
  @moduledoc """
  An ockam worker to log all messages
  """
  use Ockam.Worker

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("Printer received: #{inspect(message)}")
    {:ok, state}
  end
end
