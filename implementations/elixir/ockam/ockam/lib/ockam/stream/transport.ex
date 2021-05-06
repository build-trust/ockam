defmodule Ockam.Stream.Transport do
  @moduledoc """
  A transport using Ockam.Stream to transfer messages
  """
  use GenServer

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.Stream.Client.Publisher
  alias Ockam.Stream.Transport.Address
  alias Ockam.Stream.Transport.ConsumerForwarder

  require Logger

  @publisher_registry __MODULE__.Publishers
  @consumer_registry __MODULE__.Consumers

  @transport_message_encoder Ockam.Wire.Binary.V2

  def subscribe(options) do
    stream_name = Keyword.fetch!(options, :stream_name)

    case :ets.lookup(@consumer_registry, stream_name) do
      [] ->
        GenServer.call(__MODULE__, {:subscribe, options})

      [{_, consumer}] ->
        {:ok, consumer}
    end
  end

  def start_link(options) do
    GenServer.start_link(__MODULE__, options, name: __MODULE__)
  end

  def init(options) do
    create_registry()
    register_address_handler(options)
    {:ok, %{stream_options: options}}
  end

  def handle_call({:subscribe, options}, _from, state) do
    stream_options = Map.get(state, :stream_options, [])
    res = do_subscribe(Keyword.merge(stream_options, options))
    {:reply, res, state}
  end

  def handle_call({:create_publisher, stream_name}, _from, state) do
    stream_options = Map.get(state, :stream_options, [])
    res = do_create_stream_publisher(Keyword.put(stream_options, :stream_name, stream_name))
    {:reply, res, state}
  end

  def do_subscribe(options) do
    stream_name = Keyword.fetch!(options, :stream_name)

    forward_route = Keyword.get(options, :forward_route, [])

    case :ets.lookup(@consumer_registry, stream_name) do
      [] ->
        case ConsumerForwarder.subscribe(forward_route, options) do
          {:ok, address} ->
            register_stream_consumer(stream_name, address)

          error ->
            Logger.error("Unable to subscribe: #{inspect(stream_name)}: #{inspect(error)}")
            error
        end

      [{_, consumer}] ->
        {:ok, consumer}
    end
  end

  def do_create_stream_publisher(options) do
    stream_name = Keyword.fetch!(options, :stream_name)

    case :ets.lookup(@publisher_registry, stream_name) do
      [] ->
        case Publisher.create(options) do
          {:ok, address} ->
            register_stream_publisher(stream_name, address)

          error ->
            Logger.error(
              "Unable to create publisher for #{inspect(stream_name)}: #{inspect(error)}"
            )

            error
        end

      [{_, publisher}] ->
        {:ok, publisher}
    end
  end

  defp register_address_handler(options) do
    handler = fn msg -> route_message(msg, options) end

    with :ok <- Router.set_message_handler(Address.address_type(), handler),
         :ok <- Router.set_message_handler(Address, handler) do
      :ok
    end
  end

  def route_message(message, options) do
    [%Address{} = address | onward_route] = Message.onward_route(message)
    %Address{onward_stream: onward_stream, return_stream: return_stream} = address
    publisher = get_stream_publisher(onward_stream)

    ## TODO: This may be moved to the publisher, but it's currently expects a binary message

    case Keyword.get(options, :implicit_consumer, false) do
      true ->
        subscribe(stream_name: return_stream)

      false ->
        :ok
    end

    transport_message = %{
      onward_route: onward_route,
      return_route: [return_address(address) | Message.return_route(message)],
      payload: Message.payload(message)
    }

    case make_publisher_message(transport_message, publisher) do
      {:ok, publisher_message} ->
        Ockam.Node.send(publisher, publisher_message)

      {:error, reason} ->
        Logger.error("Encode error: #{inspect(reason)}")
    end
  end

  def make_publisher_message(transport_message, publisher) do
    with {:ok, data} <- encode_transport_message(transport_message) do
      {:ok,
       %{
         onward_route: [publisher],
         return_route: [],
         payload: Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, data)
       }}
    end
  end

  def encode_transport_message(transport_message) do
    Ockam.Wire.encode(@transport_message_encoder, transport_message)
  end

  def decode_transport_message(data) do
    Ockam.Wire.decode(@transport_message_encoder, data)
  end

  def return_address(%Address{onward_stream: onward_stream, return_stream: return_stream}) do
    %Address{onward_stream: return_stream, return_stream: onward_stream}
  end

  def get_stream_publisher(stream_name) do
    case find_stream_publisher(stream_name) do
      {:ok, publisher} ->
        publisher

      :error ->
        create_stream_publisher(stream_name)
        get_stream_publisher(stream_name)
    end
  end

  def find_stream_publisher(stream_name) do
    case :ets.lookup(@publisher_registry, stream_name) do
      [] ->
        :error

      [{^stream_name, publisher}] ->
        {:ok, publisher}
    end
  end

  def create_stream_publisher(stream_name) do
    GenServer.call(__MODULE__, {:create_publisher, stream_name})
  end

  def register_stream_publisher(stream_name, publisher) do
    :ets.insert(@publisher_registry, {stream_name, publisher})
  end

  def register_stream_consumer(stream_name, consumer) do
    :ets.insert(@consumer_registry, {stream_name, consumer})
  end

  def create_registry() do
    :ets.new(@publisher_registry, [:named_table, :public])
    :ets.new(@consumer_registry, [:named_table, :public])
  end
end
