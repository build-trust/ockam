defmodule Ockam.Router do
  @moduledoc """
  """

  import Ockam.Router.Guards

  alias Ockam.Router.Address
  alias Ockam.Router.Message
  alias Ockam.Router.MessageHandler

  @doc """
  Register a message handler for a specific address.

  The router routes messages by looking at the first address in the onward
  route of a message. If this first address is equal to the provided address
  then the provided message handler should be invoked.

  Two addresses `a1` and `a2` are considered equal if
  `Ockam.Address.type(a1) === Ockam.Address.type(a2)` and
  `Ockam.Address.value(a1) === Ockam.Address.value(a2)`
  """
  @spec register_address_handler(Address.t(), MessageHandler.t()) ::
          :ok | {:error, :already_registered}
  def register_address_handler(address, handler) when is_message_handler(handler) do
    case Registry.register(__MODULE__, {:address, address}, handler) do
      {:ok, _} -> :ok
      {:error, {:already_registered, _}} -> {:error, :already_registered}
    end
  end

  @doc """
  Unregister the message handler for the given address.
  """
  @spec unregister_address_handler(Address.t()) :: :ok
  def unregister_address_handler(address) do
    Registry.unregister(__MODULE__, {:address, address})
  end

  @doc """
  Unregister a message handler for the given address type.
  """
  @spec unregister_address_type_handler(Address.type()) :: :ok
  def unregister_address_type_handler(address_type) do
    Registry.unregister(__MODULE__, {:address_type, address_type})
  end

  @doc """
  Register a message handler for an address type.

  The router routes messages by looking at the first address in the onward
  route of a message. If this first address does not have a registed address
  specific handler then the router checks if there is a handler registered for
  the type of the address, if so then that handler will be invoked.

  The address types of two addresses `a1` and `a2` are considered equal if
  `Ockam.Address.type(a1) === Ockam.Address.type(a2)`
  """
  @spec register_address_type_handler(Address.type(), MessageHandler.t()) ::
          :ok | {:error, :already_registered}
  def register_address_type_handler(address_type, handler)
      when is_address_type(address_type) and is_message_handler(handler) do
    case Registry.register(__MODULE__, {:address_type, address_type}, handler) do
      {:ok, _} -> :ok
      {:error, {:already_registered, _}} -> {:error, :already_registered}
    end
  end

  @doc """
  Register the default message handler.
  """
  @spec register_default_handler(MessageHandler.t()) :: :ok | {:error, :already_registered}
  def register_default_handler(handler) when is_message_handler(handler) do
    case Registry.register(__MODULE__, :default_handler, handler) do
      {:ok, _} -> :ok
      {:error, {:already_registered, _}} -> {:error, :already_registered}
    end
  end

  @doc """
  """
  @spec route(Message.t()) :: :ok | {:error, :no_default_handler}
  def route(message), do: route(Message.onward_route(message), message)

  defp route([], message) do
    case Registry.lookup(__MODULE__, :default_handler) do
      [] -> {:error, :no_default_handler}
      [{_, handler}] -> handler.(message)
    end
  end

  defp route([address | _], message) do
    case Registry.lookup(__MODULE__, {:address, address}) do
      [] -> route(Address.type(address), message)
      [{_, handler}] -> handler.(message)
    end
  end

  defp route(address_type, message) when is_address_type(address_type) do
    case Registry.lookup(__MODULE__, {:address_type, address_type}) do
      [{_, handler}] ->
        handler.(message)

      [] ->
        case Registry.lookup(__MODULE__, :default_handler) do
          [] -> {:error, :no_default_handler}
          [{_, handler}] -> handler.(message)
        end
    end
  end

  # Returns a specification to start this module under a supervisor. When this
  # module is added to a supervisor, the supervisor calls child_spec to figure
  # out the specification that should be used.
  #
  # See the "Child specification" section in the `Supervisor` module for more
  # detailed information.
  @doc false
  def child_spec(options) do
    start = {__MODULE__, :start_link, [options]}
    %{id: __MODULE__, start: start, type: :worker, restart: :permanent, shutdown: 500}
  end

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def register_name(address, process_id) do
    Registry.register_name({__MODULE__, address}, process_id)
  end

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def unregister_name(address), do: Registry.unregister_name({__MODULE__, address})

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def whereis_name(address), do: Registry.whereis_name({__MODULE__, address})

  # Starts the router and links it to the current process.
  @doc false
  def start_link(_options) do
    Registry.start_link(keys: :unique, name: __MODULE__)
  end
end
