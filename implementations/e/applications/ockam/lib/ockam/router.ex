defmodule Ockam.Router do
  @moduledoc """
  Routes messages.
  """

  import Ockam.Router.Guards

  alias Ockam.Router.Address
  alias Ockam.Router.Message
  alias Ockam.Router.Storage
  alias Ockam.Telemetry

  @typedoc """
  A function that accepts an address and a message as arguments.

  It returns `:ok` or `{:error, reason}`, where `reason` can be any term.
  """
  @type message_handler() :: (Address.t(), Message.t() -> :ok | {:error, reason :: any()})

  @doc """
  Routes a message to invoke.
  """
  @spec route(Message.t()) :: :ok | {:error, reason :: any()}
  def route(message) do
    destination_address = get_destination_address_for_message(message)
    message_handler = get_message_handler_for_address(destination_address)

    invoke_message_handler(message_handler, destination_address, message)
  end

  # Returns the address to which a message would be routed. This is
  # usually the first address in the onward route of a message.
  #
  # If the onward route is empty, then the default address is returned.
  # The default address may be `nil` if no default address has been set.
  @spec get_destination_address_for_message(Message.t()) :: Address.t() | nil
  defp get_destination_address_for_message(message) do
    case Message.onward_route(message) do
      [] -> get_default_address()
      [first_address | _rest_of_the_route] -> first_address
    end
  end

  # Invokes the message handler with the message as the arg
  @spec invoke_message_handler(message_handler(), Address.t(), Message.t()) ::
          :ok | {:error, reason :: any()}

  defp invoke_message_handler(handler, address, message) when is_message_handler(handler) do
    handler.(address, message)
  end

  defp invoke_message_handler(_handler, address, message) do
    {:error, {:no_handler, address, message}}
  end

  @doc """
  Returns the default address to which a message is routed if the
  onward route of the message is empty.

  Returns `nil` if no default address has been set.
  """
  @spec get_default_address() :: Address.t() | nil

  def get_default_address, do: Storage.get(:default_address)

  @doc """
  Sets the default address to which a message is routed if the
  onward route of the message is empty.

  Returns `:ok` if the default address is successfully set.
  Returns `:error` if the default addresr could not be set.
  """
  @spec set_default_address(Address.t()) :: :ok | :error

  def set_default_address(address), do: Storage.put(:default_address, address)

  @doc """
  Unsets the default_address.

  Always returns `:ok`.
  """
  @spec unset_default_address() :: :ok

  def unset_default_address, do: Storage.delete(:default_address)

  @doc """
  Returns the handler which is invoked for messages that are destined
  for a given address.

  Returns `nil` if no handler has been set for this specific address,
  no handler has been set for the address type and there is no default
  handler.
  """
  @spec get_message_handler_for_address(Address.t()) :: message_handler() | nil

  def get_message_handler_for_address(nil), do: get_default_message_handler()

  def get_message_handler_for_address(address) do
    case Storage.get({:address, address}) do
      nil -> get_message_handler_for_address_type(Address.type(address))
      handler -> handler
    end
  end

  @doc """
  Sets the handler that should be invoked for messages that are destined for
  the given address.

  Returns `:ok` if the address specific handler is successfully set.
  Returns `:error` if the address specific handler could not be set.
  """
  @spec set_message_handler_for_address(Address.t(), message_handler()) :: :ok | :error

  def set_message_handler_for_address(address, handler) when is_message_handler(handler) do
    Storage.put({:message_handler_for_address, address}, handler)
  end

  @doc """
  Unsets message handler for the given address type.

  Always returns `:ok`.
  """
  @spec unset_message_handler_for_address(Address.t()) :: :ok

  def unset_message_handler_for_address(address) do
    Storage.delete({:message_handler_for_address, address})
  end

  @doc """
  Returns the handler which is invoked for a given address type.

  Returns `nil` if no handler is set for this specific address type.
  """
  @spec get_message_handler_for_address_type(Address.t()) :: message_handler() | nil

  def get_message_handler_for_address_type(nil), do: get_default_message_handler()

  def get_message_handler_for_address_type(address_type) do
    case Storage.get({:address_type_message_handler, address_type}) do
      nil -> get_default_message_handler()
      handler -> handler
    end
  end

  @doc """
  Sets the handler that should be invoked for messages that are destined for
  the given address.

  Returns `:ok` if the address specific handler is successfully set.
  Returns `:error` if the address specific handler could not be set.
  """
  @spec set_message_handler_for_address_type(Address.t(), message_handler()) :: :ok | :error

  def set_message_handler_for_address_type(address_type, handler)
      when is_address_type(address_type) and is_message_handler(handler) do
    Storage.put({:message_handler_for_address_type, address_type}, handler)
  end

  @doc """
  Unsets message handler for the given address type.

  Always returns `:ok`.
  """
  @spec unset_message_handler_for_address_type(Address.type()) :: :ok

  def unset_message_handler_for_address_type(address_type) when is_address_type(address_type) do
    Storage.delete({:message_handler_for_address_type, address_type})
  end

  @doc """
  Returns the default message handler which is invoked for messages that can't
  be handled by an address specific handler or an address type specific handler.

  Returns `nil` if no default message handler is set.
  """
  @spec get_default_message_handler() :: message_handler() | nil

  def get_default_message_handler, do: Storage.get(:default_message_handler)

  @doc """
  Sets the default address to which messages should be routed if the onward
  route of a message is empty.

  Returns `:ok` if the default address is successfully set.
  Returns `:error` if the default address could not be set.
  """
  @spec set_default_message_handler(message_handler()) :: :ok | :error

  def set_default_message_handler(handler) when is_message_handler(handler) do
    Storage.put(:default_message_handler, handler)
  end

  @doc """
  Unsets the default message handler.

  Always returns `:ok`.
  """
  @spec unset_default_message_handler() :: :ok

  def unset_default_message_handler, do: Storage.delete(:default_message_handler)

  @doc false
  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  defdelegate register_name(address, pid), to: Storage

  @doc false
  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  defdelegate whereis_name(address), to: Storage

  @doc false
  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  defdelegate unregister_name(address), to: Storage

  @doc false
  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  defdelegate send(address, message), to: Storage

  @doc false
  # Returns a specification to start this module under a supervisor. When this
  # module is added to a supervisor, the supervisor calls child_spec to figure
  # out the specification that should be used.
  #
  # See the "Child specification" section in the `Supervisor` module for more
  # detailed information.
  def child_spec(options) do
    start = {__MODULE__, :start_link, [options]}
    %{id: __MODULE__, start: start, type: :worker, restart: :permanent, shutdown: 500}
  end

  @doc false
  # Starts the router and links it to the current process.
  def start_link(options) do
    Telemetry.event(:ockam_router_start_link, %{}, %{})
    Storage.start_link(options)
  end
end
