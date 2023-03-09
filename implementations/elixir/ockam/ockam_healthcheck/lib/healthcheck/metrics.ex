defmodule Ockam.Healthcheck.Metrics do
  @moduledoc """
  Metrics to expose healthcheck results
  """
  alias Telemetry.Metrics

  require Logger

  def metrics() do
    [
      Metrics.last_value("ockam.healthcheck.result.status",
        event_name: [:ockam, :healthcheck, :result],
        measurement: :status,
        tags: [:target_name, :target_host, :target_port, :api_worker, :healthcheck_worker],
        tag_values: &expand_target/1
      ),
      Metrics.distribution("ockam.healthcheck.ok.duration",
        event_name: [:ockam, :healthcheck, :ok],
        measurement: :duration,
        unit: {:native, :millisecond},
        reporter_options: [buckets: [1, 10, 50, 100, 1000]],
        tags: [:target_name, :target_host, :target_port, :api_worker, :healthcheck_worker],
        tag_values: &expand_target/1
      ),
      Metrics.distribution("ockam.healthcheck.error.duration",
        event_name: [:ockam, :healthcheck, :error],
        measurement: :duration,
        unit: {:native, :millisecond},
        reporter_options: [buckets: [1, 10, 50, 100, 1000]],
        tags: [:target_host, :target_port, :api_worker, :healthcheck_worker, :reason],
        tag_values: fn meta ->
          meta = expand_target(meta)

          case Map.fetch(meta, :reason) do
            {:ok, {reason, _extra}} ->
              Map.put(meta, :reason, inspect(reason))

            {:ok, reason} when is_atom(reason) or is_binary(reason) ->
              Map.put(meta, :reason, inspect(reason))

            _other ->
              Map.put(meta, :reason, "other")
          end
        end
      )
    ]
  end

  defp expand_target(meta) do
    target = Map.get(meta, :target, %{})

    Map.merge(meta, %{
      target_name: Map.get(target, :name, ""),
      target_host: Map.get(target, :host, ""),
      target_port: Map.get(target, :port, 0),
      api_worker: Map.get(target, :api_worker, ""),
      healthcheck_worker: Map.get(target, :healthcheck_worker, "")
    })
  end
end
