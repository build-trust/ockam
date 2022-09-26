defmodule Ockam.Services.API.Tests.EchoAPI do
  @moduledoc false
  use Ockam.Services.API

  @impl true
  def handle_request(request, state) do
    {:reply, 200, request.body, state}
  end
end

defmodule Ockam.Services.API.Test do
  use ExUnit.Case

  alias Ockam.API.Client, as: ApiClient

  alias Ockam.Services.Tests.TelemetryListener

  alias Ockam.Services.API.Tests.EchoAPI

  @telemetry_table :api_test_telemetry_listener

  @telemetry_events [
    [:ockam, :api, :handle_request, :start],
    [:ockam, :api, :handle_request, :stop]
  ]

  setup_all do
    metrics_listener = TelemetryListener.start(@telemetry_table, @telemetry_events)

    on_exit(fn ->
      send(metrics_listener, :stop)
    end)

    {:ok, api} = EchoAPI.create(address: "echo_api")
    [api: api]
  end

  test "service request/response", %{api: api} do
    TelemetryListener.reset(@telemetry_table)

    body = "sample_body"
    {:ok, resp} = ApiClient.sync_request(:get, "path", body, [api])

    assert %{status: 200, body: ^body} = resp

    metrics = TelemetryListener.get_metrics(@telemetry_table)

    assert [
             {[:ockam, :api, :handle_request, :start],
              %{metadata: %{method: :get, path: "path"}}},
             {[:ockam, :api, :handle_request, :stop],
              %{
                measurements: %{duration: _duration},
                metadata: %{method: :get, path: "path", reply: true, status: 200}
              }}
           ] = Enum.sort(metrics)
  end
end
