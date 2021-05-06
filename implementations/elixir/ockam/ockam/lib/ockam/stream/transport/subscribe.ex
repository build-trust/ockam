defmodule Ockam.Stream.Transport.Subscribe do
  @moduledoc """
  Helper service to subscribe remote nodes to stream transport
  """
  use Ockam.Worker

  use Ockam.Protocol.Mapping

  require Logger

  @protocol_mapping Ockam.Protocol.Mapping.server(Ockam.Protocol.Binary)

  @impl true
  def protocol_mapping(), do: @protocol_mapping

  @impl true
  def handle_message(%{payload: payload} = message, state) do
    res =
      with {:ok, Ockam.Protocol.Binary, stream_name} <- decode_payload(payload) do
        return_route = Ockam.Message.return_route(message)
        Logger.info("Subscribe #{inspect(return_route)} to #{inspect(stream_name)}")
        ## TODO: handle errors
        Ockam.Stream.Transport.subscribe(stream_name: stream_name, forward_route: return_route)
      end

    Logger.info(inspect(res))
    {:ok, state}
  end
end
