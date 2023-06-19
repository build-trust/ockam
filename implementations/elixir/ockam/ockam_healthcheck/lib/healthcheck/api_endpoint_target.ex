defmodule Ockam.Healthcheck.APIEndpointTarget do
  @moduledoc """
  API Endpoint Healthcheck target.
  Specifies TCP host and port to connect to, secure channel listener API worker,
  healthcheck worker, path, method and optionally a Base64 encoded body for the endpoint
  """

  @enforce_keys [:name, :host, :port, :path, :method, :healthcheck_worker]
  defstruct [
    :name,
    :host,
    :port,
    :path,
    :method,
    :body,
    :healthcheck_worker,
    api_worker: "api"
  ]
end
