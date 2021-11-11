defmodule Ockam.Session.Tests.Handshake do
  @moduledoc false

  @behaviour Ockam.Session.Handshake

  def init(_options, state) do
    {:next,
     %{
       onward_route: state.init_route,
       return_route: [state.handshake_address],
       payload: "init"
     }, state}
  end

  def handle_initiator(options, _message, state) do
    reply_to = Keyword.fetch!(options, :reply_to)

    reply = %{
      onward_route: reply_to,
      return_route: [state.handshake_address],
      payload: "initiator reply"
    }

    {:ready, reply, [], state}
  end

  def handle_responder(options, message, state) do
    reply_to = Keyword.get(options, :reply_to, Ockam.Message.return_route(message))

    reply = %{
      onward_route: reply_to,
      return_route: [state.handshake_address],
      payload: "responder reply"
    }

    {:ready, reply, [], state}
  end
end

defmodule Ockam.Session.Tests.DataModule do
  @moduledoc false

  use Ockam.Worker

  @impl true
  def handle_message(_message, state) do
    {:ok, state}
  end
end
