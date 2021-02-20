if Code.ensure_loaded?(:ranch) do
  defmodule Ockam.Transport.TCP.Listener do
    @moduledoc false

    use Ockam.Worker

    alias Ockam.Message
    alias Ockam.Transport.TCP.Client
    alias Ockam.Transport.TCPAddress
    alias Ockam.Wire

    @tcp 1
    # TODO: modify this for tcp
    @wire_encoder_decoder Ockam.Wire.Binary.V2

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
      protocol_options = [packet: 2]

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

      with :ok <- Router.set_message_handler(@tcp, handler),
           :ok <- Router.set_message_handler(Ockam.Transport.TCPAddress, handler) do
        :ok
      end
    end

    defp setup_routed_message_handler(_something_else, _listener), do: :ok

    defp handle_routed_message(listener, message) do
      Node.send(listener, message)
    end

    @impl true
    def handle_message({:tcp, _socket, _from_ip, _from_port, _packet} = tcp_message, state) do
      send_over_tcp(tcp_message, state.address)
      {:ok, state}
    end

    def handle_message(message, state) do
      encode_and_send_over_tcp(message, state)
      {:ok, state}
    end

    defp encode_and_send_over_tcp(message, %{address: address}) do
      message = create_outgoing_message(message)

      with {:ok, destination, message} <- pick_destination_and_set_onward_route(message, address),
           {:ok, message} <- set_return_route(message, address),
           {:ok, encoded_message} <- Wire.encode(@wire_encoder_decoder, message),
           :ok <- send_over_tcp(encoded_message, destination) do
        :ok
      end
    end

    defp send_over_tcp(message, %{ip: ip, port: port}) do
      {:ok, pid} = Client.start_link(%{ip: ip, port: port})
      Client.send(pid, message)
    end

    defp create_outgoing_message(message) do
      %{
        onward_route: Message.onward_route(message),
        payload: Message.payload(message),
        return_route: Message.return_route(message)
      }
    end

    defp pick_destination_and_set_onward_route(message, address) do
      destination_and_onward_route =
        message
        |> Message.onward_route()
        |> Enum.drop_while(fn a -> a === address end)
        |> List.pop_at(0)

      case destination_and_onward_route do
        {nil, []} -> {:error, :no_destination}
        {%TCPAddress{} = destination, r} -> {:ok, destination, %{message | onward_route: r}}
        {{@tcp, address}, onward_route} -> deserialize_address(message, address, onward_route)
        {destination, _onward_route} -> {:error, {:invalid_destination, destination}}
      end
    end

    defp deserialize_address(message, address, onward_route) do
      case TCPAddress.deserialize(address) do
        {:error, error} -> {:error, error}
        destination -> {:ok, destination, %{message | onward_route: onward_route}}
      end
    end

    defp set_return_route(%{return_route: return_route} = message, address) do
      {:ok, %{message | return_route: [address | return_route]}}
    end

    defp default_ip, do: {127, 0, 0, 1}
    defp default_port, do: 4000
  end

  defmodule Ockam.Transport.TCP.Listener.Handler do
    @moduledoc false

    use GenServer

    @wire_encoder_decoder Ockam.Wire.Binary.V2

    def start_link(ref, transport, opts) do
      pid = :proc_lib.spawn_link(__MODULE__, :init, [[ref, transport, opts]])
      {:ok, pid}
    end

    @impl true
    def init([ref, transport, opts]) do
      {:ok, socket} = :ranch.handshake(ref, opts)
      :ok = :inet.setopts(socket, [{:active, true}, {:packet, 2}])

      address = Ockam.Node.get_random_unregistered_address()

      Ockam.Node.Registry.register_name(address, self())

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
    def handle_info({:tcp, socket, data}, %{socket: socket} = state) do
      case Ockam.Wire.decode(@wire_encoder_decoder, data) do
        {:ok, decoded} -> send_to_router(decoded)
        {:error, %Ockam.Wire.DecodeError{} = e} -> raise e
      end

      {:noreply, state}
    end

    def handle_info({:tcp_closed, socket}, %{socket: socket, transport: transport} = state) do
      transport.close(socket)
      {:stop, :normal, state}
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

    defp send_to_router(message) do
      Ockam.Router.route(message)
    end
  end
end
