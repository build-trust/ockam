defmodule Ockam.Services.TokenLeaseManager.StorageService.Memory do
  @moduledoc false
  @behaviour Ockam.Services.TokenLeaseManager.StorageService

  # alias Ockam.Services.TokenLeaseManager.Lease

  @ets __MODULE__

  defp lease_key(lease), do: {lease.issued_for, lease.id}

  @impl true
  def init(options) do
    table = :ets.new(@ets, [:public, :ordered_set])
    leases = Keyword.fetch!(options, :leases)
    :ets.insert(table, Enum.map(leases, fn l -> {lease_key(l), l} end))
    {:ok, table}
  end

  @impl true
  def save(table, lease) do
    true = :ets.insert(table, {lease_key(lease), lease})
    :ok
  end

  @impl true
  def get(table, issued_for, lease_id) do
    case :ets.lookup(table, {issued_for, lease_id}) do
      [{_, lease}] -> {:ok, lease}
      [] -> {:ok, nil}
    end
  end

  @impl true
  def remove(table, issued_for, lease_id) do
    with true <- :ets.delete(table, {issued_for, lease_id}) do
      :ok
    end
  end

  @impl true
  def get_all(table, issued_for) do
    {:ok, Enum.concat(:ets.match(table, {{issued_for, :_}, :"$1"}))}
  end
end
