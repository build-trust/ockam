if Code.ensure_loaded?(:ranch) do
  defmodule Ockam.Transport.TCP.Listener do
    @moduledoc false

    use Ockam.Worker

    alias Ockam.Message
    alias Ockam.Transport.TCP.Client
    alias Ockam.Transport.TCPAddress

    require Logger

    @doc false
    @impl true
    def setup(options, state) do
      ip = Keyword.get_lazy(options, :ip, &default_ip/0)
      state = Map.put(state, :ip, ip)

      port = Keyword.get_lazy(options, :port, &default_port/0)
      state = Map.put(state, :port, port)

      route_outgoing = Keyword.get(options, :route_outgoing, false)

      ref = make_ref()
      transport = :ranch_tcp
      transport_options = [port: port]
      protocol = __MODULE__.Handler
      protocol_options = [packet: 2, nodelay: true]

      with {:ok, _apps} <- Application.ensure_all_started(:ranch),
           :ok <- start_listener(ref, transport, transport_options, protocol, protocol_options),
           :ok <- setup_routed_message_handler(route_outgoing, state.address) do
        {:ok, state}
      end
    end

    defp start_listener(ref, transport, transport_options, protocol, protocol_options) do
      r = :ranch.start_listener(ref, transport, transport_options, protocol, protocol_options)

      case r do
        {:ok, _child} -> :ok
        {:ok, _child, _info} -> :ok
        {:error, reason} -> {:error, {:could_not_start_ranch_listener, reason}}
      end
    end

    defp setup_routed_message_handler(true, listener) do
      handler = fn message -> handle_routed_message(listener, message) end

      with :ok <- Router.set_message_handler(TCPAddress.type(), handler) do
        :ok
      end
    end

    defp setup_routed_message_handler(_something_else, _listener), do: :ok

    defp handle_routed_message(listener, message) do
      Node.send(listener, message)
    end

    @impl true
    def handle_message(message, %{address: address} = state) do
      case get_destination_and_onward_route(message, address) do
        {:ok, destination, onward_route} ->
          ## Remove tcp address from onward route
          message_to_forward =
            Map.put(create_outgoing_message(message), :onward_route, onward_route)

          ## TODO: do we want to pass a configured address?
          ## TODO: what to do with failures?
          with {:ok, client_address} <- Client.create(destination: destination) do
            Ockam.Node.send(client_address, message_to_forward)
          end

        e ->
          Logger.error(
            "Cannot forward message to tcp client: #{inspect(message)} reason: #{inspect(e)}"
          )
      end

      {:ok, state}
    end

    defp create_outgoing_message(message) do
      %{
        onward_route: Message.onward_route(message),
        payload: Message.payload(message),
        return_route: Message.return_route(message)
      }
    end

    defp get_destination_and_onward_route(message, address) do
      {dest_address, onward_route} =
        message
        |> Message.onward_route()
        |> Enum.drop_while(fn a -> a === address end)
        |> List.pop_at(0)

      with true <- TCPAddress.is_tcp_address(dest_address),
           {:ok, destination} <- TCPAddress.to_host_port(dest_address) do
        {:ok, destination, onward_route}
      else
        false ->
          {:error, {:invalid_address_type, dest_address}}

        error ->
          error
      end
    end

    defp default_ip, do: {127, 0, 0, 1}
    defp default_port, do: 4000
  end

  defmodule Ockam.Transport.TCP.Listener.Handler do
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

    @impl true
    def handle_call({:send, data}, _from, %{socket: socket, transport: transport} = state) do
      {:reply, transport.send(socket, data), state}
    end

    def handle_call(:peername, _from, %{socket: socket} = state) do
      {ip, port} = :inet.peername(socket)
      {:reply, {ip, port, self()}, state}
    end

    def send(ref, {ip, port}, data) do
      # TODO: this needs to do something other than
      # just look up by source IP and source port.
      peernames = peernames(ref)

      {_ip, _port, pid} =
        Enum.find(peernames, fn {peer_ip, peer_port, _pid} ->
          peer_ip == ip and peer_port == port
        end)

      GenServer.call(pid, {:send, data})
    end

    def peernames(ref) do
      connections = :ranch.procs(ref, :connections)

      Enum.map(connections, fn conn ->
        {:ok, {ip, port, pid}} = GenServer.call(conn, :peername)
        {ip, port, pid}
      end)
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
end
