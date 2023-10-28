defmodule Ockam.Messaging.ConfirmPipe.Receiver do
  @moduledoc """
  Receiver part of the confirm pipes.

  Receives wrapped messages with confirm refs
  Unwraps and forwards messages
  Sends confirm messages with confirm ref to the message sender
  """
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  alias Ockam.Messaging.ConfirmPipe.Wrapper

  require Logger

  @impl true
  def handle_message(message, state) do
    return_route = Message.return_route(message)
    wrapped_message = Message.payload(message)

    case Wrapper.unwrap_message(wrapped_message) do
      {:ok, ref, message} ->
        Worker.route(message, state)
        send_confirm(ref, return_route, state)
        {:ok, state}

      {:error, err} ->
        Logger.error("Error unwrapping message: #{inspect(err)}")
        {:error, err}
    end
  end

  def send_confirm(ref, return_route, state) do
    Worker.route(
      %{
        onward_route: return_route,
        return_route: [state.address],
        payload: ref_payload(ref)
      },
      state
    )
  end

  def ref_payload(ref) do
    :bare.encode(ref, :uint)
  end
end
