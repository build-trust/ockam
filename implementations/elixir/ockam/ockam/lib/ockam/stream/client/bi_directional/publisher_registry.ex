defmodule Ockam.Stream.Client.BiDirectional.PublisherRegistry do
  @moduledoc """
  Global mapping of stream pairs to PublisherProxy addresses
  """

  ## TODO: maybe use an Agent
  use GenServer

  alias Ockam.Stream.Client.BiDirectional.PublisherProxy

  def ensure_publisher(publisher_id, options) do
    ## TODO: supervise that
    ensure_registry()

    case lookup(publisher_id) do
      {:ok, address} ->
        {:ok, address}

      :error ->
        create_publisher(publisher_id, options)
    end
  end

  def ensure_registry() do
    __MODULE__.start_link([])
  end

  def create_publisher(publisher_id, options) do
    GenServer.call(__MODULE__, {:create_publisher, publisher_id, options})
  end

  def start_link(options) do
    GenServer.start_link(__MODULE__, options, name: __MODULE__)
  end

  @impl true
  def init(_options) do
    registry = create_registry()
    {:ok, %{registry: registry}}
  end

  @impl true
  def handle_call({:create_publisher, publisher_id, options}, _from, state) do
    case lookup(publisher_id) do
      {:ok, address} ->
        {:reply, {:ok, address}, state}

      :error ->
        {consumer_stream, publisher_stream, subscription_id} = publisher_id

        {:ok, address} =
          PublisherProxy.create(
            consumer_stream: consumer_stream,
            publisher_stream: publisher_stream,
            subscription_id: subscription_id,
            stream_options: options
          )

        :ok = save_publisher(publisher_id, address)
        {:reply, {:ok, address}, state}
    end
  end

  def create_registry() do
    :ets.new(__MODULE__, [:named_table, :public])
  end

  def save_publisher(publisher_id, address) do
    true = :ets.insert(__MODULE__, {publisher_id, address})
    :ok
  end

  def lookup(publisher_id) do
    case :ets.lookup(__MODULE__, publisher_id) do
      [{^publisher_id, address}] -> {:ok, address}
      [] -> :error
    end
  end
end
