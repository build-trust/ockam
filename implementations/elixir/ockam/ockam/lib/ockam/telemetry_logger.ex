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

    def attach(subscribe_events) do
      :telemetry.attach_many(
        "logger",
        subscribe_events,
        &Ockam.TelemetryLogger.handle_event/4,
        nil
      )
    end

    def detach() do
      :telemetry.detach("logger")
    end
  end
end
