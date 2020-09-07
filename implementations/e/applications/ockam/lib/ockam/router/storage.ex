defmodule Ockam.Router.Storage do
  @moduledoc false

  @registry_name __MODULE__

  def get(key) do
    case Registry.lookup(@registry_name, key) do
      [{_pid, value}] -> value
      [] -> nil
    end
  rescue
    _error -> nil
  end

  def put(key, value) do
    case Registry.register(@registry_name, key, value) do
      {:ok, _} ->
        :ok

      {:error, {:already_registered, _}} ->
        result = Registry.update_value(@registry_name, key, fn _old -> value end)

        case result do
          {_new_address, _old_address} -> :ok
          _error_ -> :error
        end
    end
  rescue
    _error -> :error
  end

  def delete(key) do
    Registry.unregister(@registry_name, key)
  rescue
    _error -> :ok
  end

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def register_name(address, pid), do: Registry.register_name({@registry_name, address}, pid)

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def whereis_name(address), do: Registry.whereis_name({@registry_name, address})

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def unregister_name(address), do: Registry.unregister_name({@registry_name, address})

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def send(address, message), do: Registry.send({@registry_name, address}, message)

  def start_link(_options), do: Registry.start_link(keys: :unique, name: @registry_name)
end
