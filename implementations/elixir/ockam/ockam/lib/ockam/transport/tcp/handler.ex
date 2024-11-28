defmodule Ockam.Transport.TCP.Handler do
  @moduledoc false

  use GenServer

  alias Ockam.Message
  alias Ockam.Telemetry
  alias Ockam.Transport.TCP.TransportMessage

  require Logger

  @address_prefix "TCP_H_"
  @active 10
  @send_timeout 30_000

  def start_link(ref, _socket, transport, opts) do
    start_link(ref, transport, opts)
  end

  def start_link(ref, transport, opts) do
    pid = :proc_lib.spawn_link(__MODULE__, :init, [[ref, transport, opts]])
    {:ok, pid}
  end

  @impl true
  def init([ref, transport, opts]) do
    {handler_options, ranch_options} = Keyword.pop(opts, :handler_options, [])

    {:ok, socket} = :ranch.handshake(ref, ranch_options)

    # Header, protocol version "1" must be the first thing exchanged.
    # It isn't send anymore after the initial exchange.
    transport.send(socket, <<1>>)

    case transport.recv(socket, 1, 5000) do
      {:ok, <<1>>} ->
        :ok =
          :inet.setopts(socket, [
            {:active, @active},
            {:send_timeout, @send_timeout},
            {:packet, 4},
            {:nodelay, true}
          ])

        {:ok, address} = Ockam.Node.register_random_address(@address_prefix, __MODULE__)

        {function_name, _} = __ENV__.function
        Telemetry.emit_event(function_name)

        authorization = Keyword.get(handler_options, :authorization, [])

        tcp_wrapper =
          Keyword.get(handler_options, :tcp_wrapper, Ockam.Transport.TCP.DefaultWrapper)

        # There are cases where we want to detect and close tcp connections after a long
        # period without activity.  Mainly to handle the case of a server receiving connections
        # from a long running client that "leak" them without closing.
        idle_timeout = Keyword.get(handler_options, :idle_timeout, :infinity)

        :gen_server.enter_loop(
          __MODULE__,
          [],
          %{
            socket: socket,
            transport: transport,
            address: address,
            authorization: authorization,
            tcp_wrapper: tcp_wrapper,
            idle_timeout: idle_timeout
          },
          {:via, Ockam.Node.process_registry(), address},
          idle_timeout
        )

      {:ok, other} ->
        Logger.warning("Unrecongnized connection header received: #{other}")
        {:stop, :normal}

      {:error, :closed} ->
        Logger.debug(
          "Connection closed by peer or otherwise lost, before receiving ockam connection header"
        )

        {:stop, :normal}

      {:error, other} ->
        # will be logged by the gen_server failing.
        {:stop, other}
    end
  end

  @impl true
  def handle_info(:timeout, %{socket: socket, transport: transport, address: address} = state) do
    # idle_timeout expired, close the socket and exit
    transport.close(socket)
    Logger.info("Closing socket transport #{inspect(address)} due to inactivity")
    {:stop, :normal, state}
  end

  def handle_info({:tcp, socket, ""}, %{socket: socket, idle_timeout: idle_timeout} = state) do
    ## Empty TCP payload - ignore
    {:noreply, state, idle_timeout}
  end

  def handle_info(
        {:tcp, socket, data},
        %{socket: socket, address: address, idle_timeout: idle_timeout} = state
      ) do
    {function_name, _} = __ENV__.function

    data_size = byte_size(data)

    Telemetry.emit_event([:tcp, :handler, :message],
      measurements: %{byte_size: data_size},
      metadata: %{address: address}
    )

    case TransportMessage.decode(data) do
      {:ok, decoded} ->
        forwarded_message =
          decoded
          |> Message.trace(address)

        send_to_router(forwarded_message)
        Telemetry.emit_event(function_name, metadata: %{name: "decoded_data"})
        {:noreply, state, idle_timeout}

      {:error, e} ->
        start_time = Telemetry.emit_start_event(function_name)
        Telemetry.emit_exception_event(function_name, start_time, e)
        {:stop, {:error, e}, state}
    end
  end

  def handle_info({:tcp_closed, socket}, %{socket: socket, transport: transport} = state) do
    transport.close(socket)
    {function_name, _} = __ENV__.function
    Telemetry.emit_event(function_name, metadata: %{name: "transport_close"})
    {:stop, :normal, state}
  end

  def handle_info({:tcp_passive, socket}, %{idle_timeout: idle_timeout} = state) do
    :ok = :inet.setopts(socket, [{:active, @active}])
    {:noreply, state, idle_timeout}
  end

  def handle_info(
        %Ockam.Message{} = message,
        %{idle_timeout: idle_timeout} = state
      ) do
    reply =
      Ockam.Worker.with_handle_message_metric(__MODULE__, message, state, fn ->
        case is_authorized(message, state) do
          :ok ->
            handle_message(message, state)

          {:error, reason} ->
            {:error, {:unauthorized, reason}}
        end
      end)

    case reply do
      {:ok, state} ->
        {:noreply, state, idle_timeout}

      {:error, reason} ->
        Logger.warning("Unauthorized message #{inspect(reason)}")
        {:noreply, state, idle_timeout}
    end
  end

  def handle_info(other, %{idle_timoeut: idle_timeout} = state) do
    Logger.warning("TCP HANDLER Received unknown message #{inspect(other)} #{inspect(state)}")
    {:noreply, state, idle_timeout}
  end

  def is_authorized(message, state) do
    Ockam.Worker.Authorization.with_state_config(message, state)
  end

  defp handle_message(
         %Ockam.Message{} = message,
         %{
           socket: socket,
           tcp_wrapper: tcp_wrapper,
           transport: transport
         } = state
       ) do
    forwarded_message = Message.forward(message)

    {:ok, encoded} = TransportMessage.encode(forwarded_message)
    :ok = tcp_wrapper.wrap_tcp_call(transport, :send, [socket, encoded])
    {:ok, state}
  end

  defp send_to_router(message) do
    ## TODO: do we want to handle that?
    Ockam.Router.route(message)
  end
end
