defmodule Ockam.Hub.Echoer do
  @moduledoc false

  use Ockam.Worker

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("NEW MESSAGE - #{inspect({message, state})}")
    {:ok, state}
  end
end
