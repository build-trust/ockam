defmodule Ockam.Services.Metrics do
  @moduledoc """
  Telemetry.Metrics configurations for Ockam Services
  """
  alias Telemetry.Metrics

  def metrics() do
    [
      Metrics.last_value("ockam.services.service",
        measurement: :count,
        event_name: [:ockam, :services, :service],
        tags: [:id, :address, :module]
      ),
      Metrics.last_value("ockam.credentials.attribute_sets.count"),
      Metrics.counter("ockam.credentials.presented.count", tags: [:identity_id]),
      Metrics.counter("ockam.credentials.verified.count", tags: [:identity_id, :attributes]),
      Metrics.counter("ockam.api.handle_request",
        event_name: [:ockam, :api, :handle_request, :start],
        measurement: :system_time,
        tags: [:address, :path, :method]
      ),
      Metrics.distribution("ockam.api.handle_request.duration",
        event_name: [:ockam, :api, :handle_request, :stop],
        measurement: :duration,
        tags: [:address, :path, :method, :status, :reply],
        unit: {:native, :millisecond},
        reporter_options: [buckets: [0.01, 0.1, 0.5, 1]]
      ),
      Metrics.counter("ockam.api.handle_request.decode_error",
        event_name: [:ockam, :api, :handle_request, :decode_error],
        tags: [:address, :status, :reply]
      ),
      Metrics.last_value("ockam.workers.secure_channels.with_credentials.count")
    ]
  end
end
