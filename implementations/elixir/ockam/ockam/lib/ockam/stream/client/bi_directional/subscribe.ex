defmodule Ockam.Protocol.Stream.BiDirectional do
  @moduledoc """
  Protocol for creating remote stream subscription
  """
  @behaviour Ockam.Protocol

  def protocol() do
    %Ockam.Protocol{
      name: "stream_bidirectional",
      request: {:struct, [stream_name: :string, subscription_id: :string]}
    }
  end
end

defmodule Ockam.Protocol.Stream.BiDirectional.EnsurePublisher do
  @moduledoc """
  Protocol for creating remote stream publisher
  """
  @behaviour Ockam.Protocol

  def protocol() do
    %Ockam.Protocol{
      name: "stream_bidirectional_publisher",
      request:
        {:struct,
         [
           publisher_stream: :string,
           consumer_stream: :string,
           subscription_id: :string
         ]}
    }
  end
end

defmodule Ockam.Stream.Client.BiDirectional.Subscribe do
  @moduledoc """
  Helper service to subscribe remote nodes to bi-deirectional stream
  """
  use Ockam.Worker
  use Ockam.Protocol.Mapping

  alias Ockam.Message
  alias Ockam.Protocol.Stream.BiDirectional, as: SubscribeProtocol
  alias Ockam.Protocol.Stream.BiDirectional.EnsurePublisher, as: PublisherProtocol
  alias Ockam.Stream.Client.BiDirectional

  require Logger

  @protocol_mapping Ockam.Protocol.Mapping.mapping([
                      {:server, SubscribeProtocol},
                      {:server, PublisherProtocol}
                    ])

  @impl true
  def protocol_mapping(), do: @protocol_mapping

  @impl true
  def setup(options, state) do
    stream_options = Keyword.fetch!(options, :stream_options)
    {:ok, Map.put(state, :stream_options, stream_options)}
  end

  @impl true
  def handle_message(message, state) do
    case decode_payload(Message.payload(message)) do
      {:ok, SubscribeProtocol, %{stream_name: stream_name, subscription_id: subscription_id}} ->
        subscription_id =
          case subscription_id do
            :undefined -> "default"
            _subscription_id -> subscription_id
          end

        BiDirectional.subscribe(stream_name, subscription_id, Map.fetch!(state, :stream_options))

      {:ok, PublisherProtocol,
       %{
         publisher_stream: publisher_stream,
         consumer_stream: consumer_stream,
         subscription_id: subscription_id
       }} ->
        {:ok, address} =
          BiDirectional.ensure_publisher(
            consumer_stream,
            publisher_stream,
            subscription_id,
            Map.fetch!(state, :stream_options)
          )

        Ockam.Router.route(%{
          onward_route: Message.return_route(message),
          return_route: [address],
          payload: "irrelevant"
        })

        {:ok, state}

      other ->
        Logger.error("Unexpected message: #{inspect(other)}")
    end

    {:ok, state}
  end
end
