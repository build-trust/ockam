defmodule Ockam.Transport.TCP.Handler do
  @moduledoc false

  use GenServer

  alias Ockam.Message
  alias Ockam.Telemetry
  alias Ockam.Transport.TCP

  require Logger

  @address_prefix "TCP_H_"

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
    :ok = :inet.setopts(socket, [{:active, true}, {:packet, 2}, {:nodelay, true}])

    {:ok, address} = Ockam.Node.register_random_address(@address_prefix, __MODULE__)

    {function_name, _} = __ENV__.function
    Telemetry.emit_event(function_name)

    authorization = Keyword.get(handler_options, :authorization, [])
    tcp_wrapper = Keyword.get(handler_options, :tcp_wrapper, Ockam.Transport.TCP.DefaultWrapper)

    :gen_server.enter_loop(
      __MODULE__,
      [],
      %{
        socket: socket,
        transport: transport,
        address: address,
        authorization: authorization,
        tcp_wrapper: tcp_wrapper
      },
      {:via, Ockam.Node.process_registry(), address}
    )
  end

  @impl true
  def handle_info({:tcp, socket, ""}, %{socket: socket} = state) do
    ## Empty TCP payload - ignore
    {:noreply, state}
  end

  def handle_info({:tcp, socket, data}, %{socket: socket, address: address} = state) do
    {function_name, _} = __ENV__.function

    data_size = byte_size(data)

    Telemetry.emit_event([:tcp, :handler, :message],
      measurements: %{byte_size: data_size},
      metadata: %{address: address}
    )

    case Ockam.Wire.decode(data, :tcp) do
      {:ok, decoded} ->
        forwarded_message =
          decoded
          |> Message.trace(address)

        send_to_router(forwarded_message)
        Telemetry.emit_event(function_name, metadata: %{name: "decoded_data"})

      {:error, %Ockam.Wire.DecodeError{} = e} ->
        start_time = Telemetry.emit_start_event(function_name)
        Telemetry.emit_exception_event(function_name, start_time, e)
        raise e
    end

    {:noreply, state}
  end

  def handle_info({:tcp_closed, socket}, %{socket: socket, transport: transport} = state) do
    transport.close(socket)
    {function_name, _} = __ENV__.function
    Telemetry.emit_event(function_name, metadata: %{name: "transport_close"})
    {:stop, :normal, state}
  end

  def handle_info(
        %Ockam.Message{} = message,
        state
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
        {:noreply, state}

      {:error, reason} ->
        Logger.warning("Unauthorized message #{inspect(reason)}")
        {:noreply, state}
    end
  end

  def handle_info(other, state) do
    Logger.warning("TCP HANDLER Received unknown message #{inspect(other)} #{inspect(state)}")
    {:noreply, state}
  end

  def is_authorized(message, state) do
    Ockam.Worker.Authorization.with_state_config(message, state)
  end

  def handle_message(
        %Ockam.Message{} = message,
        %{socket: socket, tcp_wrapper: tcp_wrapper, transport: transport} = state
      ) do
    forwarded_message = Message.forward(message)

    case Ockam.Wire.encode(forwarded_message) do
      {:ok, encoded} ->
        ## TODO: send/receive message in multiple TCP packets
        case byte_size(encoded) <= TCP.packed_size_limit() do
          true ->
            tcp_wrapper.wrap_tcp_call(transport, :send, [socket, encoded])

          false ->
            Logger.error("Message to big for TCP: #{inspect(message)}")
            raise {:message_too_big, message}
        end

      a ->
        Logger.error("TCP transport send error #{inspect(a)}")
        raise a
    end

    {:ok, state}
  end

  defp send_to_router(message) do
    ## TODO: do we want to handle that?
    Ockam.Router.route(message)
  end
end
