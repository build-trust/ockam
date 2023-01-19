defmodule Ockam.Services.TokenLeaseManager.CloudService do
  @moduledoc false
  @type lease :: map()
  @type reason :: any()
  @type token_id :: String.t()
  @type options :: Keyword.t()
  @type cloud_configuration :: Keyword.t()
  @type state :: map()
  @type creation_options :: map()

  @callback init(options) :: {:ok, cloud_configuration} | {:error, reason}
  @callback create(cloud_configuration, identity_id :: String.t(), ttl: integer()) ::
              {:ok, lease} | {:error, reason}
  @callback revoke(cloud_configuration, token_id) :: :ok | {:error, reason}
  @callback get_all(cloud_configuration) :: {:ok, [lease]} | {:error, reason}
end
