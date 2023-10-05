defmodule Ockam.Services.Metrics.TelemetryPoller do
  @moduledoc """
  Telemetry metrics reporting functions to be used with :telemetry_poller
  """
  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage

  alias Ockam.Telemetry

  require Logger

  def measurements() do
    [
      {__MODULE__, :dispatch_services, []},
      {__MODULE__, :dispatch_credential_attributes, []},
      {__MODULE__, :dispatch_channels_with_credentials, []},
      {__MODULE__, :dispatch_tcp_listeners, []},
      {__MODULE__, :dispatch_tcp_connections, []}
    ]
  end

  def dispatch_services() do
    services = Ockam.Services.list_services()

    Enum.each(services, fn service ->
      ## Reporting count because I couldn't figure out how to report no measurements
      Telemetry.emit_event([:services, :service],
        measurements: %{count: 1},
        metadata: %{id: service.id, address: service.address, module: to_string(service.module)}
      )
    end)

    ## We want poller to recover and continue polling after crashes
  catch
    type, error ->
      {type, error}
  end

  def dispatch_credential_attributes() do
    with {:ok, all_attributes} <- AttributeStorage.list_records() do
      Telemetry.emit_event([:credentials, :attribute_sets],
        measurements: %{count: Enum.count(all_attributes)}
      )
    end

    ## We want poller to recover and continue polling after crashes
  catch
    type, error ->
      {type, error}
  end

  ## TODO: update to initiators after implementing symmetrical exchange
  def dispatch_channels_with_credentials() do
    channels = Ockam.Metrics.TelemetryPoller.secure_channels()
    %{data_responders: responders} = channels

    channels_with_credentials =
      Enum.filter(responders, fn responder ->
        {:ok, remote_identity} = Ockam.SecureChannel.get_remote_identity_id(responder)

        case AttributeStorage.get_attribute_set(remote_identity) do
          {:ok, _set} -> true
          {:error, _reason} -> false
        end
      end)

    Telemetry.emit_event([:workers, :secure_channels, :with_credentials],
      measurements: %{count: Enum.count(channels_with_credentials)}
    )
  catch
    type, error ->
      {type, error}
  end

  def dispatch_tcp_listeners() do
    ranch_listeners =
      Enum.map(:ranch.info(), fn {_ref, info} ->
        port = Map.get(info, :port, 0)
        status = Map.get(info, :status, :unknown)
        {port, status}
      end)
      |> Map.new()

    configured_port =
      Application.get_env(:ockam_services, :tcp_transport, [])
      |> Keyword.get(:listen, [])
      |> Keyword.get(:port, :none)

    ranch_listeners =
      case Map.get(ranch_listeners, configured_port) do
        nil ->
          Map.put(ranch_listeners, configured_port, :missing)

        _port ->
          ranch_listeners
      end

    Enum.each(ranch_listeners, fn {port, status} ->
      live_status =
        case status do
          :running ->
            1

          _other ->
            Logger.warning(
              "Configured TCP port listener is not running: #{inspect(port)} - #{inspect(status)}"
            )

            0
        end

      Telemetry.emit_event([:tcp, :listeners],
        measurements: %{status: live_status},
        metadata: %{port: port}
      )
    end)
  end

  def dispatch_tcp_connections() do
    Enum.map(:ranch.info(), fn {_ref, info} ->
      connections = Map.get(info, :all_connections, [])
      port = Map.get(info, :port, 0)

      Telemetry.emit_event([:tcp, :connections],
        measurements: %{count: connections},
        metadata: %{port: port}
      )
    end)
  catch
    type, error ->
      {type, error}
  end
end
