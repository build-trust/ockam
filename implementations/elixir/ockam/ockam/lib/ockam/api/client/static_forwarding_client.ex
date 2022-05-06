defmodule Ockam.API.Client.StaticForwarding do
  @moduledoc """
  API client for static forwarding service API
  """

  alias Ockam.API.Response

  @doc """
  Subscribe the current worker in the static forwarding service

  Arguments:
  `alias_str` - used to create a forwarding alias
  `api_route` - route to the static forwarding API worker
  `self_address` - address of this worker (has to be registered to the current process)
  `timeout` - API timeout

  Returns:
  {:ok, forwarder_address, forwarder_route} - returns address and route to the forwarder alias
  {:error, reason}
  """

  @spec subscribe(
          alias_str :: binary(),
          api_route :: Ockam.Address.route(),
          self_address :: Ockam.Address.t(),
          timeout :: integer()
        ) ::
          {:ok, forwarder_address :: Ockam.Address.t(), forwarder_route :: Ockam.Address.route()}
          | {:error, reason :: any()}
  def subscribe(alias_str, api_route, self_address, timeout \\ 5000) do
    case Ockam.API.Client.sync_request(:post, "", alias_str, api_route, timeout, self_address) do
      {:ok, %Response{status: 200, body: forwarder_address, from_route: route}} ->
        {:ok, {:subscribed, forwarder_address, forwarder_route(route, forwarder_address)}}

      {:ok, %Response{status: status, body: error}} ->
        {:error, {:api_error, status, error}}

      other ->
        {:error, {:unexpected_response, other}}
    end
  end

  defp forwarder_route(api_route, forwarder_address) do
    Enum.take(api_route, length(api_route) - 1) ++ [forwarder_address]
  end
end
