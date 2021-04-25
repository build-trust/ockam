defmodule Ockam.TokenLeaseManager.CloudService do
  @moduledoc false
  @type lease :: map()
  @type reason :: any()
  @type token_id :: String.t()
  @type options :: Keyword.t()

  @callback configuration() :: Keyword.t()
  @callback create(options) :: {:ok, lease} | {:error, reason}
  @callback revoke(token_id) :: :ok | {:error, reason}
  @callback renew(token_id) :: :ok | {:error, reason}
  @callback get(token_id) :: {:ok, lease} | {:error, reason}
  @callback get_all() :: {:ok, [lease]} | {:error, reason}
end
