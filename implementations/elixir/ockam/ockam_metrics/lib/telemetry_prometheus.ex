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
            metrics: prepare_tags(metrics, shared_tags),
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

  def get_configured_metrics() do
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
  defp prepare_tags(metrics, shared_tags) do
    metrics
    |> add_shared_tags(shared_tags)
    |> format_tag_values()
  end

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

  defp format_tag_values(metrics) do
    Enum.map(metrics, fn metric ->
      Map.update!(metric, :tag_values, fn tag_values_fun ->
        fn md ->
          md
          |> tag_values_fun.()
          |> tags_to_string()
        end
      end)
    end)
  end

  defp tags_to_string(metadata) do
    Enum.map(metadata, fn
      {key, val} when is_atom(val) -> {key, to_string(val)}
      other -> other
    end)
    |> Map.new()
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
      last_value("ockam.workers.secure_channels.count", tags: [:type, :stage]),
      ## TODO: these metrics report relatively high cardinality
      ## these metrics need to be replaced with tracing
      counter("ockam.node.message.sent.from.count",
        event_name: [:ockam, Ockam.Node, :message, :sent],
        measurement: :count,
        tags: [:from]
      ),
      counter("ockam.node.message.sent.to.count",
        event_name: [:ockam, Ockam.Node, :message, :sent],
        measurement: :count,
        tags: [:to]
      ),
      counter("ockam.node.message.unsent",
        event_name: [:ockam, Ockam.Node, :message, :unsent],
        measurement: :count,
        tags: [:from, :to]
      ),
      ##
      counter("ockam.worker.handle_message",
        event_name: [:ockam, Ockam.Worker, :handle_message, :start],
        measurement: :system_time,
        tags: [:module]
      ),
      counter("ockam.worker.handle_message.errors",
        event_name: [:ockam, Ockam.Worker, :handle_message, :stop],
        keep: fn meta ->
          match?(%{result: :error}, meta) or match?(%{result: :stop}, meta)
        end,
        measurement: :duration,
        tags: [:module, :reason],
        tag_values: fn meta ->
          case Map.fetch(meta, :return_value) do
            {:ok, {:error, reason}} ->
              Map.put(meta, :reason, extract_reason(reason))

            {:ok, {:stop, reason, _state}} ->
              Map.put(meta, :reason, extract_reason(reason))

            _other ->
              meta
          end
        end
      ),
      distribution("ockam.worker.handle_message.duration",
        event_name: [:ockam, Ockam.Worker, :handle_message, :stop],
        measurement: :duration,
        tags: [:module, :result],
        unit: {:native, :millisecond},
        reporter_options: [buckets: [1, 10, 50, 100, 250, 500, 1000]]
      ),
      counter("ockam.worker.handle_message.unauthorized",
        event_name: [:ockam, Ockam.Worker, :handle_message, :stop],
        measurement: :duration,
        keep: fn meta ->
          match?(%{result: :unauthorized}, meta)
        end,
        tags: [:module, :reason],
        tag_values: fn meta ->
          case Map.fetch(meta, :return_value) do
            {:ok, {:error, {:unauthorized, reason}}} ->
              Map.put(meta, :reason, extract_reason(reason))

            _other ->
              meta
          end
        end
      ),
      counter("ockan.worker.init",
        event_name: [:ockam, Ockam.Worker, :init, :start],
        measurement: :system_time,
        tags: [:module]
      ),
      distribution("ockam.worker.init.duration",
        event_name: [:ockam, Ockam.Worker, :init, :stop],
        measurement: :duration,
        tags: [:module, :result],
        unit: {:native, :millisecond},
        reporter_options: [buckets: [1, 10, 50, 100, 250, 500, 1000]]
      )
      | vm_memory_metrics
    ]
  end

  defp extract_reason(reason) do
    case reason do
      atom when is_atom(atom) ->
        atom

      {atom, _explanation} when is_atom(atom) ->
        atom

      string when is_binary(string) ->
        string

      {atom, _explanation, _additional} when is_atom(atom) ->
        atom

      _other ->
        :unknown
    end
  end
end
