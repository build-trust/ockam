defmodule Ockam.Worker.Authorization do
  @moduledoc """
  Helper functions to configure worker message authorization.

  Usage:
  ```
  def is_authorized(message, _state)
    Authorization.from_addresses(message, ["one", "two"])
  end
  ```

  Pipelining helpers:
  ```
  def is_authorized(message, state)
    Authorization.from_addresses(message, ["one", "two"])
    |> Authorization.to_my_address(message, state)
  end
  ```
  """
  alias Ockam.Message

  @doc """
  Allow any messages to be handled by the worker
  """
  def allow_all(prev \\ :ok) do
    chain(prev, fn -> :ok end)
  end

  @doc """
  Deny all messages from being handled by the worker
  """
  def deny_all(prev \\ :ok) do
    chain(prev, fn -> {:error, :deny_all} end)
  end

  @doc """
  Only allow messages from a set of addresses to be handled by the worker.

  Message address is taken from return_route in that case
  """
  def from_addresses(prev, message, addresses) when is_list(addresses) do
    chain(prev, fn ->
      from_addresses(message, addresses)
    end)
  end

  def from_addresses(message, addresses) do
    case Message.return_route(message) do
      [return_address | _rest] ->
        case Enum.member?(addresses, return_address) do
          true -> :ok
          false -> {:error, {:from_address, :invalid_source, return_address}}
        end

      [] ->
        {:error, {:from_address, :empty_return_route}}
    end
  end

  @doc """
  Only allow messages sent to a certain addresses to be handled by the worker

  Message address is taken from the message onward_route
  """
  def to_addresses(prev, message, addresses) when is_list(addresses) do
    chain(prev, fn ->
      to_addresses(message, addresses)
    end)
  end

  def to_addresses(message, addresses) do
    case Message.onward_route(message) do
      [onward_address | _rest] ->
        case Enum.member?(addresses, onward_address) do
          true -> :ok
          false -> {:error, {:to_address, :invalid_destination, onward_address}}
        end

      [] ->
        {:error, {:to_address, :empty_onward_route}}
    end
  end

  @doc """
  Allow messages sent to the addresses stored in the `all_addresses` of the state
  to be handled by the worker.

  Message address is taken from the message onward_route
  """
  def to_my_address(prev \\ :ok, message, state) do
    addresses = Map.get(state, :all_addresses, [])
    to_addresses(prev, message, addresses)
  end

  @doc """
  Allow messages which have `channel: :secure_channel` in their local metadata
  to be handled by the worker.
  """
  def is_secure(prev \\ :ok, message) do
    chain(prev, fn ->
      case Message.local_metadata_value(message, :channel) do
        :secure_channel -> :ok
        ## TODO: error explanation
        other -> {:error, {:is_secure, :invalid_channel, other}}
      end
    end)
  end

  @doc """
  Allow messages which have `identity: identity` in their local metadata
  to be handled by the worker.
  """
  def from_identity(prev \\ :ok, message, identity) do
    chain(prev, fn ->
      case Message.local_metadata_value(message, :identity) do
        ^identity -> :ok
        other -> {:error, {:from_identity, :invalid_identity, other}}
      end
    end)
  end

  def chain(:ok, fun) do
    fun.()
  end

  def chain({:error, _reason} = error, _fun) do
    error
  end
end
