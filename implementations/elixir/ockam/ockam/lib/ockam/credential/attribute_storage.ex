defmodule Ockam.Credential.AttributeStorageETS do
  @moduledoc """
  Storage for attribute sets implemented with ETS.

  init() should be called once by the controlling process to create a table
  """
  alias Ockam.Credential.AttributeSet

  @table __MODULE__

  @type identity_id() :: String.t()

  @spec init() :: :ok | {:error, :table_exists}
  def init() do
    case :ets.info(@table) do
      :undefined ->
        :ets.new(@table, [:public, :named_table])
        :ok

      _table ->
        {:error, :table_exists}
    end
  end

  @spec list_records() :: {:ok, %{binary() => AttributeSet.t()}} | {:error, any()}
  def list_records() do
    with_table(fn ->
      map =
        :ets.tab2list(@table)
        |> Enum.flat_map(fn {id, attribute_set} ->
          case AttributeSet.expired?(attribute_set) do
            true -> []
            false -> [{id, attribute_set}]
          end
        end)
        |> Map.new()

      {:ok, map}
    end)
  end

  @doc """
  Retrieves the attribute set for an identity ID.
  """
  @spec get_attribute_set(identity_id()) :: {:ok, AttributeSet.t()} | {:error, any()}
  def get_attribute_set(id) do
    with_table(fn ->
      case :ets.lookup(@table, id) do
        [] ->
          {:error, :not_found}

        [{^id, attribute_set}] ->
          case AttributeSet.expired?(attribute_set) do
            true ->
              true = :ets.delete(@table, id)
              {:error, :expired}

            false ->
              {:ok, attribute_set}
          end
      end
    end)
  end

  @doc """
  Retrieve valid attributes from the table.
  Returns an empty map if not able to get attributes (for any reason)
  """
  @spec get_attributes(identity_id()) :: %{String.t() => binary()}
  def get_attributes(id) do
    case get_attribute_set(id) do
      {:ok, %{attributes: %{attributes: attributes}}} -> attributes
      {:error, _reason} -> %{}
    end
  end

  @doc """
  Save attribute set for identity id
  Current attribute set (if exists) will be overridden.
  """
  @spec put_attribute_set(identity_id(), AttributeSet.t()) :: :ok | {:error, any()}
  def put_attribute_set(id, attribute_set) do
    with_table(fn ->
      true = :ets.insert(@table, {id, attribute_set})
      :ok
    end)
  end

  def with_table(fun) do
    case :ets.info(@table) do
      :undefined -> {:error, :no_ets_table}
      _other -> fun.()
    end
  end
end
