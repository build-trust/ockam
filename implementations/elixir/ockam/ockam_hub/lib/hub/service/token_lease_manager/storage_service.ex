defmodule Ockam.TokenLeaseManager.StorageService do
  @moduledoc false
  @type lease :: Ockam.TokenLeaseManager.Lease.t()
  @type reason :: any()
  @type lease_id :: String.t()
  @type options :: Keyword.t()

  @callback save(lease) :: :ok | {:error, reason}
  @callback get(lease_id) :: {:ok, lease} | {:error, reason}
  @callback remove(lease_id) :: :ok | {:error, reason}
  @callback get_all(lease) :: {:ok, [lease]} | {:error, reason}
end
