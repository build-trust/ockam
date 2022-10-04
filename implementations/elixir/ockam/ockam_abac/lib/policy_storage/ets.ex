defmodule Ockam.ABAC.PolicyStorage.ETS do
  @moduledoc """
  Implementation of Ockam.ABAC.PolicyStorage storing data in ETS table in memory.

  Runs a gen_server holding the ETS table.
  """

  @behaviour Ockam.ABAC.PolicyStorage

  use GenServer

  alias Ockam.ABAC.ActionId
  alias Ockam.ABAC.Policy

  require Logger

  @table_name __MODULE__

  @impl true
  @spec list() :: {:ok, [Policy.t()]} | {:error, any()}
  def list() do
    case table_exists?() do
      true ->
        list = :ets.tab2list(@table_name)
        {:ok, Enum.map(list, fn {_key, policy} -> policy end)}

      false ->
        {:error, :no_table}
    end
  end

  @impl true
  @spec get_policy(ActionId.t()) :: {:ok, Policy.t()} | {:error, any()}
  def get_policy(action_id) do
    case table_exists?() do
      true ->
        case :ets.lookup(@table_name, action_id) do
          [{^action_id, policy}] -> {:ok, policy}
          [] -> {:error, :not_found}
        end

      false ->
        {:error, :no_table}
    end
  end

  @impl true
  @spec put_policy(Policy.t()) :: :ok | {:error, any()}
  def put_policy(%Policy{} = policy) do
    action_id = policy.action_id

    case table_exists?() do
      true ->
        true = :ets.insert(@table_name, {action_id, policy})
        :ok

      false ->
        {:error, :no_table}
    end
  end

  @impl true
  @spec delete_policy(ActionId.t()) :: :ok | {:error, any()}
  def delete_policy(action_id) do
    case table_exists?() do
      true ->
        true = :ets.delete(@table_name, action_id)
        :ok

      false ->
        {:error, :no_table}
    end
  end

  defp table_exists?() do
    case :ets.info(@table_name) do
      :undefined -> false
      _info -> true
    end
  end

  def start_link([]) do
    GenServer.start_link(__MODULE__, [], [])
  end

  @impl true
  def init([]) do
    :ets.new(@table_name, [:named_table, :public])
    {:ok, %{}}
  end
end
