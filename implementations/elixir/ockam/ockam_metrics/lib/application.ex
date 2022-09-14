defmodule Ockam.Metrics.Application do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Main application for Ockam Metrics
  """

  use Application

  require Logger

  @doc false
  def start(_type, _args) do
    children =
      if Application.get_env(:ockam_metrics, :prometheus_port, nil) do
        [Ockam.Metrics.Prometheus]
      else
        []
      end ++
        [
          {
            :telemetry_poller,
            [
              period: :timer.seconds(30),
              measurements: [
                {Ockam.Metrics.TelemetryPoller, :dispatch_worker_count, []},
                {Ockam.Metrics.TelemetryPoller, :dispatch_tcp_connections, []}
              ]
            ]
          }
        ] ++
        if Application.get_env(:telemetry_influxdb, :host, nil) do
          [Ockam.Metrics.TelemetryInfluxDB.child_spec()]
        else
          []
        end

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end
end
