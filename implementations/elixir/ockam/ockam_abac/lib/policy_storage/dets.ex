defmodule Ockam.ABAC.PolicyStorage.DETS do
  @moduledoc """
  Implementation of Ockam.ABAC.PolicyStorage storing data in DETS table file.

  Runs a gen_server opening the DETS table.

  Options:
  - filename: atom | string - file name to use for DETS table
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
        {:ok,
         :dets.traverse(
           @table_name,
           fn {_key, policy} ->
             {:continue, policy}
           end
         )}

      false ->
        {:error, :no_table}
    end
  end

  @impl true
  @spec get_policy(ActionId.t()) :: {:ok, Policy.t()} | {:error, any()}
  def get_policy(action_id) do
    case table_exists?() do
      true ->
        case :dets.lookup(@table_name, action_id) do
          [{^action_id, policy}] -> {:ok, policy}
          [] -> {:error, :not_found}
          {:error, reason} -> {:error, reason}
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
        :dets.insert(@table_name, {action_id, policy})

      false ->
        {:error, :no_table}
    end
  end

  @impl true
  @spec delete_policy(ActionId.t()) :: :ok | {:error, any()}
  def delete_policy(action_id) do
    case table_exists?() do
      true ->
        :dets.delete(@table_name, action_id)

      false ->
        {:error, :no_table}
    end
  end

  defp table_exists?() do
    case :dets.info(@table_name) do
      :undefined -> false
      _info -> true
    end
  end

  def start_link(args) do
    GenServer.start_link(__MODULE__, args, [])
  end

  @impl true
  def init(args) do
    file_name = make_file_name(args)
    :dets.open_file(@table_name, file: file_name)
  end

  def make_file_name(args) do
    case Keyword.fetch(args, :filename) do
      {:ok, string} when is_binary(string) ->
        to_charlist(string)

      {:ok, atom} when is_atom(atom) ->
        atom

      {:ok, charlist} when is_list(charlist) ->
        charlist

      :error ->
        @table_name
    end
  end
end
