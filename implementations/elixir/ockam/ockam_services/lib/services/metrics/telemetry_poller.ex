defmodule Ockam.Services.Metrics.TelemetryPoller do
  @moduledoc """
  Telemetry metrics reporting functions to be used with :telemetry_poller
  """
  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage

  alias Ockam.Telemetry

  def measurements() do
    [
      {__MODULE__, :dispatch_services, []},
      {__MODULE__, :dispatch_credential_attributes, []},
      {__MODULE__, :dispatch_channels_with_credentials, []}
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
    _type, _error ->
      :ok
  end

  def dispatch_credential_attributes() do
    with {:ok, all_attributes} <- AttributeStorage.list_records() do
      Telemetry.emit_event([:credentials, :attribute_sets],
        measurements: %{count: Enum.count(all_attributes)}
      )
    end

    ## We want poller to recover and continue polling after crashes
  catch
    _type, _error ->
      :ok
  end

  ## TODO: update to initiators after implementing symmetrical exchange
  def dispatch_channels_with_credentials() do
    channels = Ockam.Metrics.TelemetryPoller.secure_channels()
    %{data_responders: responders} = channels

    channels_with_credentials =
      Enum.filter(responders, fn responder ->
        remote_identity = Ockam.Identity.SecureChannel.get_remote_identity_id(responder)

        case AttributeStorage.get_attribute_set(remote_identity) do
          {:ok, _set} -> true
          {:error, _reason} -> false
        end
      end)

    Telemetry.emit_event([:workers, :secure_channels, :with_credentials],
      measurements: %{count: Enum.count(channels_with_credentials)}
    )
  end
end
