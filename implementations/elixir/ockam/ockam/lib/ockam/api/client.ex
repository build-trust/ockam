defmodule Ockam.API.Client do
  @moduledoc """
  Ockam request-response API client helper
  """

  alias Ockam.API.Request
  alias Ockam.API.Response

  @spec sync_request(
          method :: atom(),
          path :: binary(),
          body :: binary(),
          route :: [Ockam.Address.t()],
          timeout :: integer(),
          self_address :: nil | Ockam.Address.t()
        ) :: {:ok, response :: Ockam.API.Response.t()} | {:error, reason :: any()}
  def sync_request(method, path, body, route, timeout \\ 5_000, self_address \\ nil) do
    request = %Request{id: Request.gen_id(), path: path, method: method, body: body}
    payload = Request.encode!(request)

    with {:ok, message} <-
           Ockam.Workers.Call.call_on_current_process(payload, route, timeout, self_address) do
      case Response.from_message(message) do
        {:ok, response} -> {:ok, response}
        {:error, error} -> {:error, {error, message}}
      end
    end
  end
end
