defmodule Ockam.Services.TokenLeaseManager.StorageService do
  @moduledoc false
  @type lease :: Ockam.Services.TokenLeaseManager.Lease.t()
  @type reason :: any()
  @type lease_id :: String.t()
  @type options :: any()
  @type storage_conf :: map()
  @type identity_id :: String.t()

  @callback init(options) :: {:ok, any()} | {:error, reason}
  @callback save(storage_conf, lease) :: :ok | {:error, reason}
  @callback get(storage_conf, identity_id, lease_id) :: {:ok, lease} | {:error, reason}
  @callback remove(storage_conf, identity_id, lease_id) :: :ok | {:error, reason}
  @callback get_all(storage_conf, identity_id) :: {:ok, [lease]} | {:error, reason}
end
