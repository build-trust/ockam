defmodule Ockam.Transport.TCP.Handler do
  @moduledoc false

  use GenServer

  alias Ockam.Message
  alias Ockam.Telemetry

  require Logger

  @wire_encoder_decoder Ockam.Wire.Binary.V2

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

    address = Ockam.Node.get_random_unregistered_address()

    Ockam.Node.Registry.register_name(address, self())

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
  def handle_info({:tcp, socket, data}, %{socket: socket, address: address} = state) do
    {function_name, _} = __ENV__.function

    with {:ok, decoded} <- Ockam.Wire.decode(@wire_encoder_decoder, data),
         {:ok, decoded} <- set_return_route(decoded, address) do
      send_to_router(decoded)
      Telemetry.emit_event(function_name, metadata: %{name: "decoded_data"})
    else
      {:error, %Ockam.Wire.DecodeError{} = e} ->
        start_time = Telemetry.emit_start_event(function_name)
        Telemetry.emit_exception_event(function_name, start_time, e)
        raise e

      e ->
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
        %{payload: _payload} = message,
        %{transport: transport, socket: socket, address: address} = state
      ) do
    with {:ok, message} <- set_onward_route(message, address),
         {:ok, encoded} <- Ockam.Wire.encode(@wire_encoder_decoder, message) do
      transport.send(socket, encoded)
    else
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

  defp set_onward_route(message, address) do
    onward_route =
      message
      |> Message.onward_route()
      |> Enum.drop_while(fn a -> a === address end)

    {:ok, %{message | onward_route: onward_route}}
  end

  defp set_return_route(%{return_route: return_route} = message, address) do
    {:ok, %{message | return_route: [address | return_route]}}
  end

  defp send_to_router(message) do
    Ockam.Router.route(message)
  end
end
