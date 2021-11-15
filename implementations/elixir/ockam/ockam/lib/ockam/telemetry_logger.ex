if Code.ensure_loaded?(:telemetry) do
  defmodule Ockam.TelemetryLogger do
    @moduledoc """
    Logger event handler for telemetry events
    """
    require Logger

    def handle_event(event, measurements, metadata, _config) do
      Logger.info(
        "\n\n===> \n#{inspect(event)}, \n#{inspect(measurements)}, \n#{inspect(metadata)}"
      )
    end

    def attach() do
      events = [
        [:ockam, Ockam.Router, :start_link],
        [:ockam, :decode_and_send_to_router],
        [:ockam, :encode_and_send_over_udp],
        [:ockam, :init],
        [:ockam, :handle_info]
      ]

      :telemetry.attach_many("logger", events, &Ockam.TelemetryLogger.handle_event/4, nil)
    end
  end
end
