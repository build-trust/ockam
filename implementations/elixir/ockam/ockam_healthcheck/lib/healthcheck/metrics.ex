defmodule Ockam.Healthcheck.Metrics do
  @moduledoc """
  Metrics to expose healthcheck results
  """
  alias Telemetry.Metrics

  require Logger

  def metrics() do
    [
      Metrics.distribution("ockam.healthcheck.ok.duration",
        event_name: [:ockam, :healthcheck, :ok],
        measurement: :duration,
        unit: {:native, :millisecond},
        reporter_options: [buckets: [0.01, 0.1, 0.5, 1]],
        tags: [:node_host, :node_port, :api_worker, :ping_worker]
      ),
      Metrics.distribution("ockam.healthcheck.error.duration",
        event_name: [:ockam, :healthcheck, :error],
        measurement: :duration,
        unit: {:native, :millisecond},
        reporter_options: [buckets: [0.01, 0.1, 0.5, 1]],
        tags: [:node_host, :node_port, :api_worker, :ping_worker, :reason],
        tag_values: fn meta ->
          case Map.fetch(meta, :reason) do
            {:ok, reason} ->
              Map.put(meta, :reason, inspect(reason))

            _other ->
              Map.put(meta, :reason, "")
          end
        end
      )
    ]
  end
end
