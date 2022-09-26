defmodule Ockam.Services.Tests.TelemetryListener do
  @moduledoc false
  def start(table, events) do
    pid =
      spawn_link(fn ->
        :ets.new(table, [:public, :named_table, :bag])

        receive do
          :stop -> :ok
        end
      end)

    :ok =
      :telemetry.attach_many(
        to_string(table),
        events,
        fn event, measurements, metadata, _config ->
          :ets.insert(
            table,
            {event, %{measurements: measurements, metadata: metadata}}
          )
        end,
        nil
      )

    pid
  end

  def reset(table) do
    :ets.delete_all_objects(table)
  end

  def get_metrics(table) do
    :ets.tab2list(table)
  end
end
