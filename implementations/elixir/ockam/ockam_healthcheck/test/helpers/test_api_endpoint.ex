defmodule Ockam.Healthcheck.TestAPIEndpoint do
  @moduledoc false
  use Ockam.Services.API.Endpoint

  alias Ockam.API.Request
  @impl true
  def init_endpoint(_config) do
    {:ok, "STATE",
     [
       {:test, :get, "/ok", &healthcheck_ok/2},
       {:test, :get, "/error", &healthcheck_error/2}
     ]}
  end

  def healthcheck_ok(_req, _data), do: {:ok, nil}

  def healthcheck_error(_req, _data), do: {:error, "Error"}

  @impl true
  def authorize(:test, _req, _bindings), do: true
end
