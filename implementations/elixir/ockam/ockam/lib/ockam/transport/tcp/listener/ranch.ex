if Code.ensure_loaded?(:ranch) do
  defmodule Ockam.Transport.TCP.Listener.Ranch do
    @moduledoc false

    use Ockam.Worker

    @doc false
    @impl true
    def setup(options, state) do
      ip = Keyword.get_lazy(options, :ip, &default_ip/0)
      state = Map.put(state, :ip, ip)

      port = Keyword.get_lazy(options, :port, &default_port/0)
      state = Map.put(state, :port, port)

      ref = make_ref()
      transport = :ranch_tcp
      transport_options = [port: port]
      protocol = __MODULE__.Protocol
      protocol_options = []

      with {:ok, _apps} <- Application.ensure_all_started(:ranch),
           :ok <- start_listener(ref, transport, transport_options, protocol, protocol_options) do
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

    defp default_ip, do: {127, 0, 0, 1}
    defp default_port, do: 4000
  end

  defmodule Ockam.Transport.TCP.Listener.Ranch.Protocol do
    @moduledoc false

    @behaviour :ranch_protocol

    use GenServer

    def start_link(ref, transport, opts) do
      pid = :proc_lib.spawn_link(__MODULE__, :init, [{ref, transport, opts}])
      {:ok, pid}
    end

    def init({ref, transport, _opts}) do
      {:ok, socket} = :ranch.handshake(ref)
      :ok = transport.setopts(socket, [{:active, true}])
      :gen_server.enter_loop(__MODULE__, [], %{socket: socket, transport: transport})
    end

    def handle_info({:tcp, socket, data}, %{socket: socket, transport: transport} = state) do
      IO.puts("#{inspect(data)}")
      transport.send(socket, data)
      {:noreply, state}
    end

    def handle_info({:tcp_closed, socket}, %{socket: socket, transport: transport} = state) do
      IO.puts("Closing")
      transport.close(socket)
      {:stop, :normal, state}
    end
  end
end
