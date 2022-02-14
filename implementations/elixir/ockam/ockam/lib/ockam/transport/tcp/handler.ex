defmodule Ockam.Transport.TCP.Handler do
  @moduledoc false

  use GenServer

  alias Ockam.Message
  alias Ockam.Telemetry

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
    {:ok, socket} = :ranch.handshake(ref, opts)
    :ok = :inet.setopts(socket, [{:active, true}, {:packet, 2}, {:nodelay, true}])

    {:ok, address} = Ockam.Node.register_random_address(@address_prefix, __MODULE__)

    {function_name, _} = __ENV__.function
    Telemetry.emit_event(function_name)

    :gen_server.enter_loop(
      __MODULE__,
      [],
      %{
        socket: socket,
        transport: transport,
        address: address
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

    case Ockam.Wire.decode(data) do
      {:ok, decoded} ->
        forwarded_message = Message.trace_address(decoded, address)
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
        %{transport: transport, socket: socket} = state
      ) do
    forwarded_message = Message.forward(message)

    case Ockam.Wire.encode(forwarded_message) do
      {:ok, encoded} ->
        transport.send(socket, encoded)

      a ->
        Logger.error("TCP transport send error #{inspect(a)}")
        raise a
    end

    {:noreply, state}
  end

  def handle_info(other, state) do
    Logger.warn("TCP HANDLER Received unkown message #{inspect(other)} #{inspect(state)}")
    {:noreply, state}
  end

  defp send_to_router(message) do
    Ockam.Router.route(message)
  end
end
