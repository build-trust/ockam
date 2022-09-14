defmodule Ockam.Metrics.Prometheus do
  @moduledoc """
    Prometheus metrics exporter

    Expose a metrics endpoint at http://localhost:${port}/metrics that
    can be scrapped by prometheus.

    Default port if not otherwise configured is 9568.
  """

  @doc false
  def child_spec(_options) do
    configured_metrics = get_configured_metrics()

    metrics =
      case Application.get_env(:ockam_metrics, :include_node_metrics, true) do
        true ->
          node_metrics() ++ configured_metrics

        false ->
          configured_metrics
      end

    shared_tags = shared_tags()

    %{
      id: TelemetryMetricsPrometheus,
      start: {
        TelemetryMetricsPrometheus,
        :start_link,
        [
          [
            metrics: add_shared_tags(metrics, shared_tags),
            port: String.to_integer(Application.fetch_env!(:ockam_metrics, :prometheus_port))
          ]
        ]
      }
    }
  end

  defp shared_tags() do
    base_tags = %{
      hostname: System.get_env("HOSTNAME", "none"),
      namespace: System.get_env("METRICS_NAMESPACE", "none")
    }

    extra_tags = Application.get_env(:ockam_metrics, :shared_tags, %{})

    Map.merge(base_tags, extra_tags)
  end

  defp get_configured_metrics() do
    static_metrics = Application.get_env(:ockam_metrics, :metrics, [])

    dynamic_metrics =
      case Application.get_env(:ockam_metrics, :metrics_fun, nil) do
        nil ->
          []

        {module, fun, args} when is_atom(module) and is_atom(fun) and is_list(args) ->
          apply(module, fun, args)

        {fun, args} when is_function(fun) and is_list(args) ->
          fun.(args)

        fun when is_function(fun) ->
          fun.()

        other ->
          raise("Invalid metrics configuration: #{inspect(other)}")
      end

    static_metrics ++ dynamic_metrics
  end

  # The options are modified to add the base_metadata map to it, and add tags
  # with its keys to the metric, besides any other tags already defined for it.
  # TODO: ideally there should be an easier way to tell the prometheus exporter
  #       to add some static, node-level tags to all metrics exported.
  defp add_shared_tags(metrics, shared_tags) do
    shared_tag_names = Map.keys(shared_tags)

    Enum.map(metrics, fn metric ->
      metric
      |> Map.update!(:tags, fn tags -> tags ++ shared_tag_names end)
      |> Map.update!(:tag_values, fn tag_values_fun ->
        fn md -> Map.merge(tag_values_fun.(md), shared_tags) end
      end)
    end)
  end

  # Returns the list of Telemetry.Metrics to expose on prometheus format.
  # TODO: maybe define these on the apps from where they are fired from?
  def node_metrics() do
    import Telemetry.Metrics

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
      |> Enum.map(fn measure -> last_value([:vm, :memory, measure], unit: :byte) end)

    [
      last_value("ockam.workers.type.count", tags: [:type]),
      last_value("ockam.tcp.connections.count", tags: [:port]),
      last_value("vm.total_run_queue_lengths.total"),
      last_value("vm.total_run_queue_lengths.io"),
      last_value("vm.total_run_queue_lengths.cpu"),
      last_value("vm.system_counts.process",
        event_name: [:vm, :system_counts],
        measurement: :process_count
      ),
      last_value("vm.system_counts.atoms",
        event_name: [:vm, :system_counts],
        measurement: :atom_count
      ),
      last_value("vm.system_counts.ports",
        event_name: [:vm, :system_counts],
        measurement: :port_count
      ),
      counter("ockam.node.message.sent",
        event_name: [:ockam, Ockam.Node, :message, :sent],
        measurement: :count,
        tags: [:from, :to]
      ),
      counter("ockam.node.message.unsent",
        event_name: [:ockam, Ockam.Node, :message, :unsent],
        measurement: :count,
        tags: [:from, :to]
      ),
      counter("ockam.worker.handle_message",
        event_name: [:ockam, Ockam.Worker, :handle_message, :start],
        measurement: :system_time,
        tags: [:address, :module]
      ),
      distribution("ockam.worker.handle_message.duration",
        event_name: [:ockam, Ockam.Worker, :handle_message, :stop],
        measurement: :duration,
        tags: [:address, :module, :result],
        unit: {:native, :millisecond},
        reporter_options: [buckets: [0.01, 0.1, 0.5, 1]]
      ),
      counter("ockan.worker.init",
        event_name: [:ockam, Ockam.Worker, :init, :start],
        measurement: :system_time,
        tags: [:address, :module]
      ),
      distribution("ockam.worker.init.duration",
        event_name: [:ockam, Ockam.Worker, :init, :stop],
        measurement: :duration,
        tags: [:address, :module, :result],
        unit: {:native, :millisecond},
        reporter_options: [buckets: [0.01, 0.1, 0.5, 1]]
      )
      | vm_memory_metrics
    ]
  end
end
