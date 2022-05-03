defmodule Ockam.CloudNode do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Main application for Ockam Cloud Node
  """

  use Application

  require Logger

  @doc false
  def start(_type, _args) do
    Logger.info("Starting Ockam Cloud Node.")
    schedule_specs = cleanup_schedule(Application.get_env(:ockam_cloud_node, :cleanup))

    children =
      [
        {
          :telemetry_poller,
          [
            period: :timer.seconds(30),
            measurements: [
              {Ockam.CloudNode.Metrics.TelemetryPoller, :dispatch_worker_count, []},
              {Ockam.CloudNode.Metrics.TelemetryPoller, :dispatch_tcp_connections, []}
            ]
          ]
        }
      ] ++
        schedule_specs ++
        if Application.get_env(:telemetry_influxdb, :host, nil) do
          [Ockam.CloudNode.Metrics.TelemetryInfluxDB.child_spec()]
        else
          []
        end

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  defp cleanup_schedule(config) do
    crontab = Keyword.get(config, :crontab)
    idle_timeout = Keyword.get(config, :idle_timeout)
    cleanup_kafka_topics = Keyword.get(config, :cleanup_kafka_topics)

    case {crontab, idle_timeout} do
      {tab, timeout} when is_binary(tab) and is_integer(timeout) ->
        case Crontab.CronExpression.Parser.parse(tab) do
          {:ok, _} ->
            [
              %{
                id: "cleanup",
                start:
                  {SchedEx, :run_every,
                   [Ockam.CloudNode.Cleanup, :cleanup_all, [timeout, cleanup_kafka_topics], tab]}
              }
            ]

          other ->
            Logger.info("Invalid cleanup crontab: #{inspect(tab)} : #{inspect(other)}. Ignoring")
            []
        end

      _other ->
        Logger.info(
          "Invalid cleanup config: #{inspect(crontab)} : #{inspect(idle_timeout)}. Ignoring"
        )

        []
    end
  end
end
