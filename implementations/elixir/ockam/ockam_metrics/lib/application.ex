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
        telemetry_poller()

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  def telemetry_poller() do
    measurements = get_poller_measurements()
    period = Application.get_env(:ockam_metrics, :poller_period, :timer.seconds(30))

    [
      {
        :telemetry_poller,
        [
          period: period,
          measurements: measurements
        ]
      }
    ]
  end

  def get_poller_measurements() do
    ockam_measurements = [
      {Ockam.Metrics.TelemetryPoller, :dispatch_worker_count, []},
      {Ockam.Metrics.TelemetryPoller, :dispatch_api_worker_count, []},
      {Ockam.Metrics.TelemetryPoller, :dispatch_secure_channels_count, []},
      {Ockam.Metrics.TelemetryPoller, :dispatch_tcp_connections, []}
    ]

    config_measurements = Application.get_env(:ockam_metrics, :poller_measurements, [])
    ockam_measurements ++ config_measurements
  end
end
