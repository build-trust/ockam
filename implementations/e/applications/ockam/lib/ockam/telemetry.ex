defmodule Ockam.Telemetry do
  @moduledoc false

  @doc false
  def start(event, metadata \\ %{}, measurements \\ %{}) do
    start_time = System.monotonic_time()

    measurements = Map.merge(measurements, %{system_time: System.system_time()})
    :telemetry.execute([:ockam, event, :start], measurements, metadata)

    start_time
  end

  @doc false
  def stop(event, start_time, metadata \\ %{}, measurements \\ %{}) do
    end_time = System.monotonic_time()
    measurements = Map.merge(measurements, %{duration: end_time - start_time})

    :telemetry.execute([:ockam, event, :stop], measurements, metadata)
  end

  @doc false
  def event(event, measurements, metadata) do
    :telemetry.execute([:ockam, event], measurements, metadata)
  end
end
