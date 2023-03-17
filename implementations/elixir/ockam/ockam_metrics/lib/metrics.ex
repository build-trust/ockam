defmodule Ockam.Metrics do
  @moduledoc """
  Ockam base metrics definitions in Telemetry.Metrics format to be used in prometheus exporter
  """
  alias Telemetry.Metrics

  def node_metrics() do
    vm_metrics() ++ ockam_workers_metrics() ++ ockam_message_metrics()
  end

  def vm_metrics() do
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
      |> Enum.map(fn measure -> Metrics.last_value([:vm, :memory, measure], unit: :byte) end)

    [
      Metrics.last_value("vm.total_run_queue_lengths.total"),
      Metrics.last_value("vm.total_run_queue_lengths.io"),
      Metrics.last_value("vm.total_run_queue_lengths.cpu"),
      Metrics.last_value("vm.system_counts.process",
        event_name: [:vm, :system_counts],
        measurement: :process_count
      ),
      Metrics.last_value("vm.system_counts.atoms",
        event_name: [:vm, :system_counts],
        measurement: :atom_count
      ),
      Metrics.last_value("vm.system_counts.ports",
        event_name: [:vm, :system_counts],
        measurement: :port_count
      )
      | vm_memory_metrics
    ]
  end

  def ockam_workers_metrics() do
    [
      Metrics.last_value("ockam.workers.type.count", tags: [:type]),
      Metrics.last_value("ockam.tcp.connections.count", tags: [:port]),
      Metrics.last_value("ockam.workers.secure_channels.count", tags: [:type, :stage]),
      Metrics.counter("ockan.worker.init",
        event_name: [:ockam, Ockam.Worker, :init, :start],
        measurement: :system_time,
        tags: [:module]
      ),
      Metrics.distribution("ockam.worker.init.duration",
        event_name: [:ockam, Ockam.Worker, :init, :stop],
        measurement: :duration,
        tags: [:module, :result],
        unit: {:native, :millisecond},
        reporter_options: [buckets: [1, 10, 50, 100, 250, 500, 1000]]
      )
    ]
  end

  def ockam_message_metrics() do
    [
      Metrics.counter("ockam.node.message.sent",
        event_name: [:ockam, Ockam.Node, :message, :sent],
        measurement: :count
      ),
      Metrics.counter("ockam.node.message.unsent",
        event_name: [:ockam, Ockam.Node, :message, :unsent],
        measurement: :count,
        tags: [:from, :to]
      ),
      ##
      Metrics.counter("ockam.worker.handle_message",
        event_name: [:ockam, Ockam.Worker, :handle_message, :start],
        measurement: :system_time,
        tags: [:module]
      ),
      Metrics.counter("ockam.worker.handle_message.errors",
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
      Metrics.distribution("ockam.worker.handle_message.duration",
        event_name: [:ockam, Ockam.Worker, :handle_message, :stop],
        measurement: :duration,
        tags: [:module, :result],
        unit: {:native, :millisecond},
        reporter_options: [buckets: [1, 10, 50, 100, 250, 500, 1000]]
      ),
      Metrics.counter("ockam.worker.handle_message.unauthorized",
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
      )
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
