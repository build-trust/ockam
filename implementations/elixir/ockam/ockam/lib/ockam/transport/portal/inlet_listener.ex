defmodule Ockam.Transport.Portal.InletListener do
  @moduledoc """
  GenServer implementing the Inlet TCP listener
  It's a GenServer just to make it simple to be added
  to a supervision tree.
  """

  use GenServer
  require Logger

  @typedoc """
  TCP listener options
  - ip: t::inet.ip_address() - IP address to listen on
  - port: t:integer() - port to listen on
  - peer_route: route to outlet
  """
  @type options :: Keyword.t()

  def start_link(options) do
    GenServer.start_link(__MODULE__, options)
  end

  @impl true
  def init(options) do
    ip = Keyword.get_lazy(options, :ip, &default_ip/0)
    port = Keyword.get_lazy(options, :port, &default_port/0)
    peer_route = options[:peer_route]
    Logger.info("Starting inlet listener on #{inspect(ip)}:#{port}. Peer: #{inspect(peer_route)}")

    {:ok, lsocket} =
      :gen_tcp.listen(port, [:binary, {:active, false}, {:ip, ip}, {:reuseaddr, true}])

    spawn_link(fn -> loop(lsocket, peer_route) end)
    {:ok, %{listen_socket: lsocket, peer_route: peer_route}}
  end

  defp loop(lsocket, peer_route) do
    case :gen_tcp.accept(lsocket) do
      {:ok, socket} ->
        ## TODO: pass authorization
        {:ok, worker} =
          Ockam.Transport.Portal.InletWorker.create(
            peer_route: peer_route,
            ## TODO: setting a controller process could be moved to start_link function
            ## of the inlet worker. Until then it doesn't make sense
            ## to restart the inlet worker
            restart_type: :temporary
          )

        case Ockam.Node.whereis(worker) do
          nil ->
            raise "Worker #{inspect(worker)} not found"

          pid ->
            :ok = :gen_tcp.controlling_process(socket, pid)
            GenServer.cast(pid, {:takeover, socket})
            loop(lsocket, peer_route)
        end

      {:error, :closed} ->
        :ok
    end
  end

  def default_ip, do: {0, 0, 0, 0}
  def default_port, do: 3000
end
