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

  require Logger

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
    Logger.debug("check from addresses #{inspect(message)} #{inspect(addresses)}")

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
  def from_secure_channel(prev \\ :ok, message, _state) do
    Logger.debug("check is secure #{inspect(message)}")

    chain(prev, fn ->
      case Message.local_metadata(message) do
        %{source: :channel, channel: :secure_channel} ->
          :ok

        %{source: :channel, channel: other} ->
          {:error, {:from_secure_channel, :invalid_channel, other}}

        other ->
          {:error, {:from_secure_channel, :invalid_metadata, other}}
      end
    end)
  end

  @doc """
  Allow messages which have `channel: :identity_secure_channel` in their local metadata
  to be handled by the worker.
  """
  def from_identiy_secure_channel(prev \\ :ok, message, _state) do
    Logger.debug("check from identity channel #{inspect(message)}")

    chain(prev, fn ->
      case Message.local_metadata(message) do
        %{source: :channel, channel: :identity_secure_channel} ->
          :ok

        %{source: :channel, channel: other} ->
          {:error, {:from_identiy_secure_channel, :invalid_channel, other}}

        other ->
          {:error, {:from_identiy_secure_channel, :invalid_metadata, other}}
      end
    end)
  end

  @doc """
  Allow messages which have `identity: identity` in their local metadata
  to be handled by the worker.
  """
  def from_identity(prev \\ :ok, message, identity_id) do
    chain(prev, fn ->
      case Message.local_metadata_value(message, :identity_id) do
        ^identity_id -> :ok
        other -> {:error, {:from_identity, :invalid_identity, other}}
      end
    end)
  end

  def is_local(prev \\ :ok, message, _state) do
    chain(prev, fn ->
      case Message.local_metadata_value(message, :source) do
        :local -> :ok
        other -> {:error, {:is_local, :message_source_not_local, other}}
      end
    end)
  end

  @doc """
  Combine multiple authorization steps in the state :authorization field

  See `with_config/3`
  """
  def with_state_config(prev, message, state) do
    chain(prev, fn ->
      with_state_config(message, state)
    end)
  end

  def with_state_config(message, state) do
    config = Map.get(state, :authorization, [:to_my_address])

    with_config(config, message, state)
  end

  @doc """
  Combine multiple authorization steps in config

  config can be either:
  - a list of steps
  - a map of %{address => list of steps}

  When steps are in a list - they are checked to all addresses.
  When steps are in a map - only the current address (first in onward_route) steps are checked

  Each step can be:
  - function :: atom - a function from `Ockam.Worker.Authorization` taking message and state as arguments
  - {function :: atom, args :: list} - a function from `Ockam.Worker.Authorization` taking args
  - {module :: atom, function :: atom} - a function form module taking message and state as arguments
  - {module :: atom, function :: atom, args :: list} - function taking args

  If args contain atoms `:message` or `:state`, they are replaced
  with the checked message or the current state of the worker

  config `:to_my_address` is same as `{:to_my_address, [:message, :state]}` and
  `{Ockam.Worker.Authorization, :to_my_address, [:message, :state]}`
  """
  def with_config(config, message, state) do
    case config do
      list when is_list(list) ->
        expand_config(config, message, state) |> check_with_config()

      map when is_map(map) ->
        [destination | _] = Message.onward_route(message)

        case Map.get(config, destination) do
          nil ->
            :ok

          address_config ->
            expand_config(address_config, message, state) |> check_with_config()
        end
    end
  end

  defp check_with_config(prev \\ :ok, config)

  defp check_with_config(prev, []) do
    prev
  end

  defp check_with_config(prev, [{m, f, a} | rest]) do
    chain(prev, fn ->
      apply(m, f, a) |> check_with_config(rest)
    end)
  end

  def expand_config(config) when is_list(config) do
    expand_config(config, :message, :state)
  end

  def expand_config(config) when is_map(config) do
    config
    |> Enum.map(fn {k, v} when is_list(v) ->
      {k, expand_config(v)}
    end)
    |> Map.new()
  end

  defp expand_config(config, message, state) do
    Enum.map(config, &expand_config_fun(&1, message, state))
  end

  defp expand_config_fun(function, message, state) when is_atom(function) do
    args = expand_args([:message, :state], message, state)
    {__MODULE__, function, args}
  end

  defp expand_config_fun({function, args}, message, state)
       when is_atom(function) and is_list(args) do
    args = expand_args(args, message, state)
    {__MODULE__, function, args}
  end

  defp expand_config_fun({module, function}, message, state)
       when is_atom(module) and is_atom(function) do
    args = [message, state]
    {module, function, args}
  end

  defp expand_config_fun({module, function, args}, message, state)
       when is_atom(module) and is_atom(function) and is_list(args) do
    args = expand_args(args, message, state)
    {module, function, args}
  end

  defp expand_args(args, message, state) do
    Enum.map(args, fn
      :message -> message
      :state -> state
      other -> other
    end)
  end

  def chain(:ok, fun) do
    fun.()
  end

  def chain({:error, _reason} = error, _fun) do
    error
  end
end
