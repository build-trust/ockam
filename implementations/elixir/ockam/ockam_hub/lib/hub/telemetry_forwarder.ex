defmodule Ockam.Hub.TelemetryForwarder do
  @moduledoc false

  def forward(handler_name, event_name, node_name, process_name) do
    handler = fn ev, mes, met, opt ->
      send({process_name, node_name}, {:telemetry, {ev, mes, met, opt}})
    end

    :telemetry.attach(handler_name, event_name, handler, nil)
  end
end
