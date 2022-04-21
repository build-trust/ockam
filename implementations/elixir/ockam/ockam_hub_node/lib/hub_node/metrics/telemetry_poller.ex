defmodule Ockam.HubNode.Metrics.TelemetryPoller do
  @moduledoc """
  Telemetry poller callbacks to collect information about workers and transports
  """

  ## TODO: maybe move this to `ockam` app?

  alias Ockam.Telemetry

  def dispatch_worker_count() do
    Enum.map(workers_by_type(), fn {type, workers} ->
      type_str = format_worker_type(type)

      Telemetry.emit_event([:workers, :type],
        measurements: %{count: Enum.count(workers)},
        metadata: %{type: type_str}
      )

      Enum.each(workers, fn address ->
        Telemetry.emit_event([:workers, :address],
          measurements: %{count: 1},
          metadata: %{type: type_str, address: address}
        )
      end)
    end)
  end

  def dispatch_tcp_connections() do
    Enum.map(:ranch.info(), fn {_ref, info} ->
      connections = Map.get(info, :all_connections, [])
      port = Map.get(info, :port, 0)

      Telemetry.emit_event([:tcp, :connections],
        measurements: %{count: connections},
        metadata: %{port: port}
      )
    end)
  end

  defp format_worker_type(nil) do
    "Other"
  end

  defp format_worker_type(module) do
    to_string(module)
  end

  @spec workers_by_type() :: [{module(), address :: String.t()}]
  def workers_by_type() do
    Ockam.Node.list_workers()
    |> Enum.group_by(fn {_address, _pid, module} -> module end, fn {address, _pid, _modules} ->
      address
    end)
    |> Map.new()
  end
end
