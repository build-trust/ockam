defmodule Ockam.Metrics.Exporter do
  defmodule MyPlug do
    import Plug.Conn

    def init(options) do
      # initialize options
      options
    end

    def call(conn, _opts) do
      body = TelemetryMetricsPrometheus.Core.scrape()

      conn
      |> put_resp_content_type("text/plain")
      |> send_resp(200, body)
    end
  end

  def child_spec(_options) do
    port = String.to_integer(Application.fetch_env!(:ockam_metrics, :prometheus_port))
    {Plug.Cowboy, plug: MyPlug, scheme: :http, options: [port: port]}
  end
end
