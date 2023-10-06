defmodule Ockam.Stream.Client.BiDirectional.PublisherProxy do
  @moduledoc """
  Publisher proxy worker to add return_stream and encode Ockam messages to binary
  Uses internal Stream.Client.Publisher through which it sends messages
  """
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Stream.Client.Publisher

  require Logger

  @publisher_prefix "STB_P_"

  @impl true
  def address_prefix(_options), do: "STB_PP_"

  @impl true
  def setup(options, state) do
    consumer_stream = Keyword.fetch!(options, :consumer_stream)
    publisher_stream = Keyword.fetch!(options, :publisher_stream)
    stream_options = Keyword.fetch!(options, :stream_options)

    ## TODO: setup is in :continue now, so :init is not needed anymore
    send(self(), {:init, publisher_stream, stream_options})

    {:ok, Map.merge(state, %{consumer_stream: consumer_stream})}
  end

  @impl true
  def handle_info({:init, publisher_stream, stream_options}, state) do
    {:ok, publisher_address} =
      Publisher.create(
        Keyword.merge(stream_options,
          stream_name: publisher_stream,
          address_prefix: @publisher_prefix
        )
      )

    {:noreply, Map.put(state, :publisher_address, publisher_address)}
  end

  @impl true
  def handle_message(%{payload: _} = message, %{publisher_address: _} = state) do
    %{
      consumer_stream: consumer_stream,
      address: self_address,
      publisher_address: publisher_address
    } = state

    [^self_address | onward_route] = Message.onward_route(message)
    forwarded_message = %{message | onward_route: onward_route}

    {:ok, encoded_message} =
      Ockam.Stream.Client.BiDirectional.encode_message(%{
        message: forwarded_message,
        return_stream: consumer_stream
      })

    binary_message =
      Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, encoded_message)

    ## TODO: should we forward metadata here?
    Ockam.Worker.route(
      %{
        payload: binary_message,
        onward_route: [publisher_address],
        return_route: []
      },
      state
    )

    {:ok, state}
  end

  def handle_message(%Ockam.Message{payload: _} = message, state) do
    ## Delay message processing
    send(self(), message)
    {:ok, state}
  end
end
