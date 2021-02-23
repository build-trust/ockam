defmodule Ockam.Hub.Service.Forward do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Hub.Service.Forward.Inbox
  alias Ockam.Hub.Service.Forward.Outbox
  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("MESSAGE: #{inspect(message)}")
    forward_route = Message.return_route(message)

    with {:ok, inbox} <- Inbox.create(forward_route: forward_route),
         outbox <- Inbox.outbox_address(inbox),
         {:ok, _} <- Outbox.create(address: outbox, inbox_address: state.address),
         :ok <- send_reply(inbox, message, state) do
      {:ok, state}
    end
  end

  def send_reply(inbox_address, message, state) do
    reply = %{
      onward_route: Message.return_route(message),
      return_route: [state.address],
      payload: inbox_address
    }

    Logger.info("REPLY: #{inspect(reply)}")
    Router.route(reply)
  end
end
