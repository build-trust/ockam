defmodule Ockam.Transport.UDP.Listener do
  @moduledoc false

  use GenServer

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Telemetry
  alias Ockam.Transport.UDPAddress
  alias Ockam.Wire
  alias Ockam.Worker

  require Logger

  def start_link(options) do
    GenServer.start_link(__MODULE__, options)
  end

  def send_message(listener, message) do
    GenServer.cast(listener, {:send_message, message})
  end

  def child_spec(options) do
    id = process_id(options)

    %{
      id: id,
      start: {__MODULE__, :start_link, [options]}
    }
  end

  def process_id(options) do
    ip = Keyword.get_lazy(options, :ip, &default_ip/0)
    port = Keyword.get_lazy(options, :port, &default_port/0)

    "UDP_LISTENER_#{UDPAddress.format_ip_port(ip, port)}"
  end

  @doc false
  @impl true
  def init(options) do
    ip = Keyword.get_lazy(options, :ip, &default_ip/0)
    port = Keyword.get_lazy(options, :port, &default_port/0)

    udp_open_options = [:binary, :inet, {:ip, ip}, {:active, true}]

    with {:ok, socket} <- :gen_udp.open(port, udp_open_options),
         :ok <- setup_routed_message_handler(self()) do
      {:ok, %{socket: socket}}
    end
  end

  defp setup_routed_message_handler(listener) do
    handler = fn message ->
      handle_transport_message(listener, message)
    end

    Router.set_message_handler(UDPAddress.type(), handler)
  end

  defp handle_transport_message(listener, message) do
    send_message(listener, message)
  end

  @doc false
  @impl true
  def handle_info({:udp, _socket, _from_ip, _from_port, _packet} = udp_message, state) do
    ## TODO: use from_ip and from_port to route messages back
    case decode_and_send_to_router(udp_message, state) do
      {:ok, state} ->
        {:noreply, state}

      {:error, error} ->
        {:stop, {:error, error}, state}
    end
  end

  @impl true
  def handle_cast({:send_message, message}, state) do
    case encode_and_send_over_udp(message, state) do
      {:ok, state} ->
        {:noreply, state}

      {:error, error} ->
        {:stop, {:error, error}, state}
    end
  end

  defp decode_and_send_to_router(udp_message, state) do
    {function_name, _} = __ENV__.function
    {:udp, _socket, from_ip, from_port, packet} = udp_message

    case Wire.decode(packet, :udp) do
      {:ok, decoded} ->
        Telemetry.emit_event(function_name, metadata: %{name: "successfully_decoded"})

        message =
          decoded
          |> Message.trace(UDPAddress.new(from_ip, from_port))

        with :ok <- Worker.route(message, state) do
          {:ok, state}
        end

      {:error, reason} ->
        {:error, Telemetry.emit_event(function_name, metadata: %{name: reason})}
    end
  end

  defp encode_and_send_over_udp(message, %{socket: socket} = state) do
    {function_name, _} = __ENV__.function

    with {:ok, destination, message} <- pick_destination_and_set_onward_route(message),
         {:ok, encoded_message} <- Wire.encode(message),
         :ok <- :gen_udp.send(socket, destination.ip, destination.port, encoded_message) do
      Telemetry.emit_event(function_name, metadata: %{name: "successfully_encoded_and_sent"})
      {:ok, state}
    else
      {:error, reason} ->
        {:error, Telemetry.emit_event(function_name, metadata: %{name: reason})}
    end
  end

  defp pick_destination_and_set_onward_route(message) do
    {dest_address, onward_route} =
      message
      |> Message.onward_route()
      |> List.pop_at(0)

    with true <- UDPAddress.is_udp_address(dest_address),
         {:ok, {ip, port}} <- UDPAddress.to_ip_port(dest_address) do
      {:ok, %{ip: ip, port: port}, %{message | onward_route: onward_route}}
    else
      false ->
        {:error, {:invalid_destination, dest_address}}

      error ->
        error
    end
  end

  defp default_ip do
    Application.get_env(:ockam, :default_ip, {0, 0, 0, 0})
  end

  defp default_port do
    Application.get_env(:ockam, :default_port, 5000)
  end
end
