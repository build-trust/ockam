defmodule Ockam.Healthcheck.Target do
  @moduledoc """
  Healthcheck target.
  Specifies TCP host and port to connect to,
  secure channel listener API worker
  and healthcheck ping endpoint
  """

  @enforce_keys [:name, :host, :port, :crontab]
  defstruct [
    :name,
    :host,
    :port,
    :crontab,
    api_worker: "api",
    healthcheck_worker: "healthcheck"
  ]
end
