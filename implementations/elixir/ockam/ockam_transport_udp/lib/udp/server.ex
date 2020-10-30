defmodule Ockam.Transport.UDP.Server do
  @moduledoc false

  # use GenServer, makes this module a GenServer.
  #
  # Among other things, it adds the `child_spec/1` function which returns a
  # specification to start this module under a supervisor. When this module is
  # added to a supervisor, the supervisor calls child_spec to figure out the
  # specification that should be used.
  #
  # See the "Child specification" section in the `Supervisor` module for more
  # detailed information.
  #
  # The `@doc` annotation immediately preceding `use GenServer` below
  # is attached to the generated `child_spec/1` function. Since we don't
  # want `child_spec/1` in our Transport module docs, `@doc false` is set here.

  @doc false
  use GenServer

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Telemetry
  alias Ockam.Transport.UDPAddress
  alias Ockam.Wire

  @wire_encoder_decoder Ockam.Wire.Binary.V1

  @doc false
  def route(message) do
    __MODULE__ |> GenServer.whereis() |> send(message)
  end

  @doc false
  # Starts the transport process linked to the current process
  def start_link(options) when is_list(options) do
    GenServer.start_link(__MODULE__, options, name: __MODULE__)
  end

  @doc false
  @impl true
  def init(options) do
    ip = Keyword.get_lazy(options, :ip, &default_ip/0)
    port = Keyword.get_lazy(options, :port, &default_port/0)

    udp_open_options = [:binary, :inet, {:ip, ip}, {:active, true}]
    udp_address = %UDPAddress{ip: ip, port: port}

    with :ok <- Router.set_message_handler(2, &route/1),
         {:ok, socket} <- :gen_udp.open(port, udp_open_options) do
      {:ok, %{socket: socket, udp_address: udp_address}}
    end
  end

  @doc false
  @impl true

  def handle_info({:udp, _socket, _from_ip, _from_port, _packet} = udp_message, state) do
    metadata = %{message: udp_message}
    start_time = Telemetry.emit_start_event([__MODULE__, :incoming], metadata: metadata)

    return_value = decode_and_send_to_router(udp_message, state)

    metadata = Map.put(metadata, :return_value, return_value)
    Telemetry.emit_stop_event([__MODULE__, :incoming], start_time, metadata: metadata)

    {:noreply, state}
  end

  def handle_info(message, state) do
    metadata = %{message: message}
    start_time = Telemetry.emit_start_event([__MODULE__, :outgoing], metadata: metadata)

    return_value = encode_and_send_over_udp(message, state)

    metadata = Map.put(metadata, :return_value, return_value)
    Telemetry.emit_stop_event([__MODULE__, :outgoing], start_time, metadata: metadata)

    {:noreply, state}
  end

  defp decode_and_send_to_router(udp_message, _state) do
    {:udp, _socket, _from_ip, _from_port, packet} = udp_message

    with {:ok, decoded} <- Wire.decode(@wire_encoder_decoder, packet),
         :ok <- Router.route(decoded) do
      :ok
    end
  end

  defp encode_and_send_over_udp(message, %{socket: socket, udp_address: address}) do
    message = create_outgoing_message(message)

    with {:ok, destination, message} <- pick_destination_and_set_onward_route(message, address),
         {:ok, message} <- set_return_route(message, address),
         {:ok, encoded_message} <- Wire.encode(@wire_encoder_decoder, message),
         :ok <- :gen_udp.send(socket, destination.ip, destination.port, encoded_message) do
      :ok
    end
  end

  defp create_outgoing_message(message) do
    %{
      onward_route: Message.onward_route(message),
      return_route: Message.return_route(message),
      payload: Message.payload(message)
    }
  end

  defp pick_destination_and_set_onward_route(message, address) do
    r =
      message
      |> Message.onward_route()
      |> Enum.drop_while(fn a -> a === address end)
      |> List.pop_at(0)

    case r do
      {nil, []} ->
        {:error, :no_destination}

      {%UDPAddress{} = destination, onward_route} ->
        {:ok, destination, %{message | onward_route: onward_route}}

      {{2, address}, onward_route} ->
        case UDPAddress.deserialize(address) do
          {:error, error} -> {:error, error}
          destination -> {:ok, destination, %{message | onward_route: onward_route}}
        end

      {destination, _onward_route} ->
        {:error, {:invalid_destination, destination}}
    end
  end

  defp set_return_route(%{return_route: return_route} = message, address) do
    {:ok, %{message | return_route: [address | return_route]}}
  end

  defp default_ip do
    Application.get_env(:ockam_transport_udp, :default_ip, {127, 0, 0, 1})
  end

  defp default_port do
    Application.get_env(:ockam_transport_udp, :default_port, 5000)
  end
end
