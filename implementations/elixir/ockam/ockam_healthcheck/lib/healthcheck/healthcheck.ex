defmodule Ockam.Healthcheck do
  @moduledoc """
  Healthcheck implementation
  """

  alias Ockam.Identity.SecureChannel
  alias Ockam.Message
  alias Ockam.Telemetry

  require Logger

  def check_node(node_host, node_port, api_worker, ping_worker, timeout \\ 5000) do
    node_address = Ockam.Transport.TCPAddress.new(node_host, node_port)
    api_route = [node_address, api_worker]
    {:ok, identity} = get_healthcheck_identity()
    start_time = System.os_time(:millisecond)

    case ping_node(api_route, identity, ping_worker, timeout) do
      :ok ->
        report_check_ok(node_host, node_port, api_worker, ping_worker, start_time)
        :ok

      {:error, reason} ->
        report_check_failed(node_host, node_port, api_worker, ping_worker, reason, start_time)
        {:error, reason}
    end
  end

  def ping_node(api_route, identity, worker, timeout) do
    ## Improve error reporting for healthcheck
    ## report secure channel and worker errors differently
    case SecureChannel.create_channel(
           route: api_route,
           identity: identity,
           key_exchange_timeout: 10_000
         ) do
      {:ok, channel} ->
        {:ok, me} = Ockam.Node.register_random_address()
        ref = inspect(make_ref())
        Ockam.Router.route(ref, [channel, worker], [me])

        result =
          receive do
            %Message{
              onward_route: [^me],
              return_route: _return_route,
              payload: ^ref
            } ->
              ## We can validate more info from the message return route and
              ## the local metadata, such as remote identity
              :ok
          after
            timeout ->
              {:error, :timeout}
          end

        Ockam.Node.whereis(channel) |> GenServer.stop(:shutdown)
        Ockam.Node.unregister_address(me)
        result

      {:error, {:worker_init, _worker, reason}} ->
        {:error, channel_init_error(reason)}

      {:error, reason} ->
        {:error, reason}
    end
  after
    cleanup_tcp_connections()
  end

  defp channel_init_error(
         {:handler_error, reason, _message,
          {Ockam.Transport.TCP, :handle_transport_message, [[]]}}
       ) do
    {:tcp_connection_error, reason}
  end

  defp channel_init_error(reason) do
    reason
  end

  ## This is a temporary solution to leaking TCP transport connections
  ## TODO: come up with a better approach to TCP connection cleanup
  def cleanup_tcp_connections() do
    Ockam.Node.list_workers()
    |> Enum.filter(fn {_addr, _pid, mod} -> mod == Ockam.Transport.TCP.Client end)
    |> Enum.each(fn {_addr, pid, _mod} -> GenServer.stop(pid, :shutdown) end)
  end

  def report_check_ok(node_host, node_port, api_worker, ping_worker, start_time) do
    duration = System.os_time(:millisecond) - start_time

    Telemetry.emit_event([:healthcheck, :ok],
      measurements: %{duration: duration},
      metadata: %{
        node_host: node_host,
        node_port: node_port,
        api_worker: api_worker,
        ping_worker: ping_worker
      }
    )
  end

  def report_check_failed(node_host, node_port, api_worker, ping_worker, reason, start_time) do
    duration = System.os_time(:millisecond) - start_time

    Telemetry.emit_event([:healthcheck, :error],
      measurements: %{duration: duration},
      metadata: %{
        node_host: node_host,
        node_port: node_port,
        api_worker: api_worker,
        ping_worker: ping_worker,
        reason: reason
      }
    )
  end

  def get_healthcheck_identity() do
    with {:ok, data} <- File.read(identity_path()),
         {:ok, identity} <- Ockam.Identity.make_identity(data),
         {:ok, _sig} <- Ockam.Identity.create_signature(identity, "") do
      {:ok, identity}
    else
      _other ->
        generate_identity()
    end
  end

  defp generate_identity() do
    identity_module = Application.get_env(:ockam, :identity_module)

    with {:ok, identity, _identity_id} <- Ockam.Identity.create(identity_module),
         :ok <- File.mkdir_p(Path.dirname(identity_path())),
         :ok <- File.write(identity_path(), Ockam.Identity.get_data(identity)) do
      {:ok, identity}
    end
  end

  defp identity_path() do
    Path.join(Application.fetch_env!(:ockam_healthcheck, :storage_path), "identity")
  end
end
