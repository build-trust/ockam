defmodule Ockam.Healthcheck do
  @moduledoc """
  Healthcheck implementation
  """

  alias Ockam.Healthcheck.Target
  alias Ockam.Identity.SecureChannel
  alias Ockam.Message
  alias Ockam.Telemetry

  require Logger

  @key_exchange_timeout 10_000

  def check_targets() do
    targets = Application.get_env(:ockam_healthcheck, :targets, [])
    ## TODO: parallel check for multiple targets
    Enum.each(targets, &Ockam.Healthcheck.check_target/1)
  end

  def check_target(
        %Target{} = target,
        timeout \\ 5000
      ) do
    start_time = System.os_time(:millisecond)

    case ping_target(target, timeout) do
      :ok ->
        report_check_ok(target, start_time)
        :ok

      {:error, reason} ->
        report_check_failed(target, reason, start_time)
        {:error, reason}
    end
  end

  def ping_target(target, timeout) do
    %{host: host, port: port, api_worker: api_worker, healthcheck_worker: healthcheck_worker} =
      target

    with_tcp(host, port, fn conn ->
      with_channel(conn, api_worker, fn channel ->
        {:ok, me} = Ockam.Node.register_random_address()
        ref = inspect(make_ref())
        Ockam.Router.route(ref, [channel, healthcheck_worker], [me])

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

        Ockam.Node.unregister_address(me)

        result
      end)
    end)
  end

  defp with_conn(conn_fun, op_fun, cleanup_fun, error_type) do
    case conn_fun.() do
      {:ok, conn} ->
        try do
          op_fun.(conn)
        catch
          _type, reason ->
            {:error, {error_type, reason}}
        after
          cleanup_fun.(conn)
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp with_tcp(host, port, fun) do
    with_conn(
      fn -> connect_tcp(host, port) end,
      fun,
      fn conn -> Ockam.Node.stop(conn) end,
      :tcp_connection_error
    )
  end

  defp connect_tcp(host, port) do
    ## TODO: use start_link?
    case Ockam.Transport.TCP.Client.create(host: host, port: port) do
      {:ok, conn} ->
        {:ok, conn}

      {:error, {:worker_init, _worker, reason}} ->
        {:error, {:tcp_connection_error, reason}}

      {:error, reason} ->
        {:error, {:tcp_connection_error, reason}}
    end
  end

  defp with_channel(tcp_conn, api_worker, fun) do
    with_conn(
      fn -> connect_secure_channel(tcp_conn, api_worker) end,
      fun,
      fn chan -> Ockam.Identity.SecureChannel.disconnect(chan) end,
      :secure_channel_error
    )
  end

  defp connect_secure_channel(tcp_conn, api_worker) do
    api_route = [tcp_conn, api_worker]

    with {:ok, identity, vault_name} <- get_healthcheck_identity() do
      Logger.debug("Identity: #{inspect(identity)}")

      case SecureChannel.create_channel(
             route: api_route,
             identity: identity,
             vault_name: vault_name,
             ## TODO: make this configurable
             key_exchange_timeout: @key_exchange_timeout
           ) do
        {:ok, channel} ->
          {:ok, channel}

        {:error,
         {:worker_init, _worker,
          {:handler_error, reason, _message,
           {Ockam.Transport.TCP, :handle_transport_message, [[]]}}}} ->
          {:error, {:tcp_connection_error, reason}}

        {:error, {:worker_init, _worker, reason}} ->
          {:error, {:secure_channel_error, reason}}

        {:error, reason} ->
          {:error, {:secure_channel_error, reason}}
      end
    end
  end

  defp log_healthcheck(message, duration, target) do
    Logger.debug(
      message <>
        " for target #{inspect(target)} " <>
        "duration: #{inspect(duration)}"
    )
  end

  def report_check_ok(target, start_time) do
    duration = System.os_time(:millisecond) - start_time

    log_healthcheck("Healthcheck OK", duration, target)

    Telemetry.emit_event([:healthcheck, :result],
      measurements: %{status: 1},
      metadata: %{
        target: target
      }
    )

    Telemetry.emit_event([:healthcheck, :ok],
      measurements: %{duration: duration},
      metadata: %{
        target: target
      }
    )
  end

  def report_check_failed(target, reason, start_time) do
    duration = System.os_time(:millisecond) - start_time

    log_healthcheck(
      "Healthcheck ERROR: #{inspect(reason)}",
      duration,
      target
    )

    Telemetry.emit_event([:healthcheck, :result],
      measurements: %{status: 0},
      metadata: %{
        target: target
      }
    )

    Telemetry.emit_event([:healthcheck, :error],
      measurements: %{duration: duration},
      metadata: %{
        target: target,
        reason: reason
      }
    )
  end

  @spec get_healthcheck_identity() ::
          {:ok, identity :: Ockam.Identity.t(), vault_name :: binary() | nil}
          | {:error, reason :: any()}
  defp get_healthcheck_identity() do
    case Application.get_env(:ockam_healthcheck, :identity_source) do
      :function ->
        function =
          Application.get_env(:ockam_healthcheck, :identity_function, &generate_identity/0)

        identity_from_function(function)

      :file ->
        file = Application.get_env(:ockam_healthcheck, :identity_file)
        identity_from_file(file)
    end
  end

  defp identity_from_function(function) when is_function(function, 0) do
    function.()
  end

  defp identity_from_function(not_function) do
    {:error, {:invalid_identity_function, not_function}}
  end

  defp identity_from_file(file) do
    case File.read(file) do
      {:ok, data} ->
        with {:ok, identity} <- Ockam.Identity.make_identity(data) do
          {:ok, identity, nil}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  def generate_identity() do
    with {:ok, identity, _identity_id} <- Ockam.Identity.create() do
      {:ok, identity, nil}
    end
  end
end
