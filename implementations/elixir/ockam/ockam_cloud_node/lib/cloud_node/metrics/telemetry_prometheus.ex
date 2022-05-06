defmodule Ockam.CloudNode.Metrics.Prometheus do
  @moduledoc """
    Prometheus metrics exporter

    Expose a metrics endpoint at http://localhost:${port}/metrics that
    can be scrapped by prometheus.

    Default port if not otherwise configured is 9568.
  """

  @doc false
  def child_spec(_opts) do
    %{
      id: TelemetryMetricsPrometheus,
      start: {
        TelemetryMetricsPrometheus,
        :start_link,
        [
          [
            metrics: metrics(),
            port: String.to_integer(Application.fetch_env!(:ockam_cloud_node, :prometheus_port))
          ]
        ]
      }
    }
  end

  # Given a metric definition, build the corresponding Telemetry.Metrics struct.
  # The options are modified to add the base_metadata map to it, and add tags
  # with its keys to the metric, besides any other tags already defined for it.
  # TODO: ideally there should be an easier way to tell the prometheus exporter
  #       to add some static, node-level tags to all metrics exported.
  defp make_metric({f, name}, base_metadata), do: make_metric({f, name, []}, base_metadata)

  defp make_metric({f, name, opts}, base_metadata) do
    base_tags = Map.keys(base_metadata)

    f.(
      name,
      opts
      |> Keyword.update(:tags, base_tags, fn tags -> tags ++ base_tags end)
      |> Keyword.put(:tag_values, fn metadata -> Map.merge(metadata, base_metadata) end)
    )
  end

  # Returns the list of Telemetry.Metrics to expose on prometheus format.
  # TODO: maybe define these on the apps from where they are fired from?
  defp metrics do
    import Telemetry.Metrics

    base_metadata = %{
      hostname: System.get_env("HOSTNAME", "none"),
      namespace: System.get_env("METRICS_NAMESPACE", "none")
    }

    vm_memory_metrics =
      [
        :total,
        :processes,
        :processes_used,
        :system,
        :atom,
        :atom_used,
        :binary,
        :code,
        :ets
      ]
      |> Enum.map(fn measure -> {&last_value/2, [:vm, :memory, measure], unit: :byte} end)

    metrics_def = [
      {&last_value/2, "ockam.workers.type.count", tags: [:type]},
      {&last_value/2, "ockam.tcp.connections.count", tags: [:port]},
      {&last_value/2, "vm.total_run_queue_lengths.total"},
      {&last_value/2, "vm.total_run_queue_lengths.io"},
      {&last_value/2, "vm.total_run_queue_lengths.cpu"},
      {&last_value/2, "vm.system_counts.process",
       event_name: [:vm, :system_counts], measurement: :process_count},
      {&last_value/2, "vm.system_counts.atoms",
       event_name: [:vm, :system_counts], measurement: :atom_count},
      {&last_value/2, "vm.system_counts.ports",
       event_name: [:vm, :system_counts], measurement: :port_count},
      {&counter/2, "ockam.node.message.sent",
       event_name: [:ockam, Ockam.Node, :message, :sent], measurement: :count, tags: [:from, :to]},
      {&counter/2, "ockam.node.message.unsent",
       event_name: [:ockam, Ockam.Node, :message, :unsent],
       measurement: :count,
       tags: [:from, :to]},
      {&counter/2, "ockam.worker.handle_message",
       event_name: [:ockam, Ockam.Worker, :handle_message, :start],
       measurement: :system_time,
       tags: [:address, :module]},
      {&distribution/2, "ockam.worker.handle_message.duration",
       event_name: [:ockam, Ockam.Worker, :handle_message, :stop],
       measurement: :duration,
       tags: [:address, :module, :result],
       unit: {:native, :millisecond},
       reporter_options: [buckets: [0.01, 0.1, 0.5, 1]]},
      {&counter/2, "ockan.worker.init",
       event_name: [:ockam, Ockam.Worker, :init, :start],
       measurement: :system_time,
       tags: [:address, :module]},
      {&distribution/2, "ockam.worker.init.duration",
       event_name: [:ockam, Ockam.Worker, :init, :stop],
       measurement: :duration,
       tags: [:address, :module, :result],
       unit: {:native, :millisecond},
       reporter_options: [buckets: [0.01, 0.1, 0.5, 1]]}
      | vm_memory_metrics
    ]

    metrics_def |> Enum.map(fn mdef -> make_metric(mdef, base_metadata) end)
  end
end
