defmodule Ockam.CloudNode.Metrics.TelemetryInfluxDB do
  @moduledoc """
  Telemetry InfluxDB metric reporting configuration
  """
  def child_spec() do
    %{
      id: TelemetryInfluxDB,
      start: {
        TelemetryInfluxDB,
        :start_link,
        [
          [
            version: :v2,
            protocol: :http,
            reporter_name: "Ockam Cloud Node",
            host: Application.get_env(:telemetry_influxdb, :host) || "http://127.0.0.1",
            port: String.to_integer(Application.get_env(:telemetry_influxdb, :port) || "8086"),
            bucket: Application.get_env(:telemetry_influxdb, :bucket) || "ockam_hub",
            org: Application.get_env(:telemetry_influxdb, :org) || "ockam",
            token: Application.get_env(:telemetry_influxdb, :token) || "TOKEN NOT CONFIGURED",
            tags: %{
              hostname: System.get_env("HOSTNAME", "none"),
              namespace: System.get_env("METRICS_NAMESPACE", "none")
            },
            events: [
              %{
                name: [:vm, :memory],
                metadata_tag_keys: [
                  :total,
                  :processes,
                  :processes_used,
                  :system,
                  :atom,
                  :atom_used,
                  :binary,
                  :code,
                  :ets,
                  :maximum
                ]
              },
              %{
                name: [:ockam, :workers, :type],
                metadata_tag_keys: [:type]
              },
              %{
                name: [:ockam, :tcp, :connections],
                metadata_tag_keys: [:port]
              },
              %{
                name: [:vm, :total_run_queue_lengths],
                metadata_tag_keys: [:total, :cpu, :io]
              },
              %{
                name: [:vm, :system_counts],
                metadata_tag_keys: [:process_count, :atom_count, :port_count]
              },
              %{
                name: [:ockam, Ockam.Node, :message, :sent],
                metadata_tag_keys: [:from, :to]
              },
              %{
                name: [:ockam, Ockam.Node, :message, :unsent],
                metadata_tag_keys: [:from, :to]
              },
              %{
                name: [:ockam, Ockam.Worker, :handle_message, :start],
                metadata_tag_keys: [:address, :module]
              },
              %{
                name: [:ockam, Ockam.Worker, :handle_message, :stop],
                metadata_tag_keys: [:address, :module, :result]
              },
              %{
                name: [:ockam, Ockam.Worker, :init, :start],
                metadata_tag_keys: [:address, :module]
              },
              %{
                name: [:ockam, Ockam.Worker, :init, :stop],
                metadata_tag_keys: [:address, :module, :result]
              }
            ]
          ]
        ]
      }
    }
  end
end
