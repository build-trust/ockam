defmodule Ockam.Transport.UDP.Listener do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Telemetry
  alias Ockam.Transport.UDPAddress
  alias Ockam.Wire

  @wire_encoder_decoder Ockam.Wire.Binary.V2

  @doc false
  @impl true
  def setup(options, state) do
    ip = Keyword.get_lazy(options, :ip, &default_ip/0)
    port = Keyword.get_lazy(options, :port, &default_port/0)

    udp_open_options = [:binary, :inet, {:ip, ip}, {:active, true}]
    udp_address = UDPAddress.new(ip, port)

    route_outgoing = Keyword.get(options, :route_outgoing, false)

    with {:ok, socket} <- :gen_udp.open(port, udp_open_options),
         :ok <- setup_routed_message_handler(route_outgoing, state.address) do
      state = Map.put(state, :socket, socket)
      state = Map.put(state, :udp_address, udp_address)
      {:ok, state}
    end
  end

  defp setup_routed_message_handler(true, listener) do
    handler = fn message -> handle_routed_message(listener, message) end

    with :ok <- Router.set_message_handler(UDPAddress.type(), handler) do
      :ok
    end
  end

  defp setup_routed_message_handler(_something_else, _listener), do: :ok

  defp handle_routed_message(listener, message) do
    Node.send(listener, message)
  end

  @doc false
  @impl true

  def handle_message({:udp, _socket, _from_ip, _from_port, _packet} = udp_message, state) do
    ## TODO: use from_ip and from_port to route messages back
    decode_and_send_to_router(udp_message, state)
  end

  def handle_message(message, state) do
    encode_and_send_over_udp(message, state)
  end

  defp decode_and_send_to_router(udp_message, state) do
    {function_name, _} = __ENV__.function
    {:udp, _socket, _from_ip, _from_port, packet} = udp_message

    with {:ok, decoded} <- Wire.decode(@wire_encoder_decoder, packet),
         :ok <- Router.route(decoded) do
      Telemetry.emit_event(function_name, metadata: %{name: "successfully_decoded"})
      {:ok, state}
    else
      {:error, reason} -> {:error, Telemetry.emit_event(function_name, metadata: %{name: reason})}
    end
  end

  defp encode_and_send_over_udp(message, %{socket: socket, udp_address: address} = state) do
    message = create_outgoing_message(message)
    {function_name, _} = __ENV__.function

    with {:ok, destination, message} <- pick_destination_and_set_onward_route(message, address),
         {:ok, message} <- set_return_route(message, address),
         {:ok, encoded_message} <- Wire.encode(@wire_encoder_decoder, message),
         :ok <- :gen_udp.send(socket, destination.ip, destination.port, encoded_message) do
      Telemetry.emit_event(function_name, metadata: %{name: "successfully_encoded_and_sent"})
      {:ok, state}
    else
      {:error, reason} -> {:error, Telemetry.emit_event(function_name, metadata: %{name: reason})}
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
    {dest_address, onward_route} =
      message
      |> Message.onward_route()
      |> Enum.drop_while(fn a -> a === address end)
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

  defp set_return_route(%{return_route: return_route} = message, address) do
    {:ok, %{message | return_route: [address | return_route]}}
  end

  defp default_ip do
    Application.get_env(:ockam, :default_ip, {127, 0, 0, 1})
  end

  defp default_port do
    Application.get_env(:ockam, :default_port, 5000)
  end
end
