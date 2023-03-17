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
          Ockam.Metrics.node_metrics() ++ configured_metrics

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
    Application.get_env(:ockam_metrics, :shared_tags, %{})
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
end
