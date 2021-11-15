if Code.ensure_loaded?(:telemetry) do
  defmodule Ockam.TelemetryLogger do
    require Logger

    def handle_event(event, measurements, metadata, _config) do
      Logger.info(
        "\n\n===> \n#{inspect(event)}, \n#{inspect(measurements)}, \n#{inspect(metadata)}"
      )
    end
  end
end
