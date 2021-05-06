defmodule Ockam.Stream.Client.BiDirectional do
  @moduledoc """
  A node-wide subscription to a stream via Stream.Client.Consumer

  On consumption creates publishers (PublishProxy) for return_stream in the messages
  """

  alias Ockam.Message

  alias Ockam.Stream.Client.BiDirectional.PublisherRegistry
  alias Ockam.Stream.Client.Consumer

  @transport_message_encoder Ockam.Wire.Binary.V2

  @doc """
  Create bidirectional consumer.
  Consumer will handle messages with handle_message/4

  Returns consumer worker address
  """
  def subscribe(stream_name, subscription_id \\ "default", stream_options) do
    message_handler = fn data ->
      handle_message(data, stream_name, subscription_id, stream_options)
    end

    consumer_options =
      Keyword.merge(
        stream_options,
        stream_name: stream_name,
        client_id: subscription_id,
        message_handler: message_handler
      )

    {:ok, _consumer_address} = Consumer.create(consumer_options)
  end

  @doc """
  Message handling for bidirectional messages
  Decodes message and return stream from data.
  Creates a return stream publisher id it doesn't exist
  Routes the message locally with return publisher address in return route
  """
  def handle_message(data, consumer_stream, subscription_id, stream_options) do
    {:ok, %{return_stream: publisher_stream, message: message}} = decode_message(data)

    {:ok, publisher_address} =
      ensure_publisher(
        consumer_stream,
        publisher_stream,
        subscription_id,
        stream_options
      )

    forwarded_message = %{
      message
      | return_route: [publisher_address | Message.return_route(message)]
    }

    Ockam.Router.route(forwarded_message)
  end

  @doc """
  Create bidirectional publisher

  The publisher will encode messages with return stream when sending them to stream
  """
  def ensure_publisher(consumer_stream, publisher_stream, subscription_id \\ "default", options) do
    publisher_id = {consumer_stream, publisher_stream, subscription_id}

    ## TODO: make it a part of consumer
    PublisherRegistry.ensure_publisher(publisher_id, options)
  end

  @bare_message {:struct, [return_stream: :string, message: :data]}

  def encode_message(%{return_stream: stream, message: message}) do
    {:ok, wire_message} = Ockam.Wire.encode(@transport_message_encoder, message)
    :bare.encode(%{return_stream: stream, message: wire_message}, @bare_message)
  end

  def decode_message(data) do
    case :bare.decode(data, @bare_message) do
      {:ok, %{return_stream: stream, message: wire_message}, ""} ->
        {:ok, message} = Ockam.Wire.decode(@transport_message_encoder, wire_message)
        {:ok, %{return_stream: stream, message: message}}

      other ->
        {:error, other}
    end
  end
end
