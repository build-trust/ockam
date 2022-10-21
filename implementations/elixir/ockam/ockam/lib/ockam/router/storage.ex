defmodule Ockam.Router.Storage do
  @moduledoc """
  Provides storage for the router's internal state.
  """

  @typedoc "The type representing a key that can be used to store a value."
  @type key :: any()

  @typedoc "The type representing a value that can be stored."
  @type value :: any()

  # use the module name as the name of the agent we'll use for storing state.
  @agent_name __MODULE__

  @doc """
  Starts the storage process and links it to the current process.
  """
  @spec start_link(options :: any()) ::
          {:ok, pid()} | {:error, reason :: {:already_started, pid()} | any()}

  def start_link(_options) do
    initial_state = %{}
    Agent.start_link(fn -> initial_state end, name: @agent_name)
  end

  @doc """
  Gets the `value` for a specific `key` from storage.

  If key has value in storage, `value` is returned.
  If key does not have a value in storage, `nil` is returned.
  """
  @spec get(key()) :: value() | nil

  def get(key) do
    Agent.get(@agent_name, fn state -> Map.get(state, key) end)
  end

  @doc """
  Puts the given `value` under `key` in storage.

  If key is successfully set, `:ok` is returned.
  If a value already exists for this key, it will be overwritten, `:ok` is returned.
  """
  @spec put(key(), value()) :: :ok

  def put(key, value) do
    Agent.update(@agent_name, fn state -> Map.put(state, key, value) end)
  end

  @doc """
  Delete the entry in storage for a specific `key`.

  If a value exists for this key, it is removed and `:ok` is returned.
  If a value does not exists for this key, `:ok` is returned.
  """
  @spec delete(key()) :: :ok

  def delete(key) do
    Agent.update(@agent_name, fn state -> Map.delete(state, key) end)
  end
end
