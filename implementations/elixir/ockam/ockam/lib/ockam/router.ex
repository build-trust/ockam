defmodule Ockam.Router do
  @moduledoc """
  Routes messages.

  The default message handler is invoked for a message if the onward route of
  the message is empty or if the first address in the onward route has a type
  for which there is no address type specific handler.
  """

  import Ockam.RoutableAddress, only: [is_address_type: 1]
  import Ockam.Router.MessageHandler, only: [is_message_handler: 1]

  alias Ockam.Message
  alias Ockam.RoutableAddress
  alias Ockam.Router.MessageHandler
  alias Ockam.Router.Storage

  alias Ockam.Telemetry

  @doc """
  Routes the given message.
  """
  @spec route(Message.t()) :: :ok | {:error, reason :: any()}

  def route(message) do
    metadata = %{message: message}
    start_time = Telemetry.emit_start_event([__MODULE__, :route], metadata: metadata)

    return_value = pick_and_invoke_handler(message)

    metadata = Map.put(metadata, :return_value, return_value)
    Telemetry.emit_stop_event([__MODULE__, :route], start_time, metadata: metadata)

    return_value
  end

  defp pick_and_invoke_handler(message) do
    first_address = message |> Message.onward_route() |> List.first()
    handler_type = if first_address, do: RoutableAddress.type(first_address), else: :default

    case get_message_handler(handler_type) do
      nil -> {:error, {:handler_not_set, handler_type, message}}
      {:error, reason} -> {:error, reason}
      handler -> invoke_handler(handler, message)
    end
  end

  defp invoke_handler(handler, message) when is_function(handler, 1) do
    case handler.(message) do
      {:error, error} -> {:error, {:handler_error, error, message, handler}}
      _anything_else -> :ok
    end
  end

  @doc """
  Returns the the handler for an address type or the default message handler.

  Returns `nil` if no message handler is set for the provided address type and
  there is also no `:default` handler set.
  """
  @spec get_message_handler(:default | RoutableAddress.type()) ::
          MessageHandler.t() | nil | {:error, reason :: any()}

  def get_message_handler(:default), do: Storage.get(:default_message_handler)

  def get_message_handler(address_type) when RoutableAddress.is_address_type(address_type) do
    case Storage.get({:address_type_message_handler, address_type}) do
      nil -> get_message_handler(:default)
      handler -> handler
    end
  end

  @doc """
  Sets the default message handler or the handler for an address type.

  Returns `:ok` if the handler is successfully set.
  """
  @spec set_message_handler(:default | RoutableAddress.type(), MessageHandler.t()) :: :ok

  def set_message_handler(:default, handler) when is_message_handler(handler) do
    Storage.put(:default_message_handler, handler)
  end

  def set_message_handler(address_type, handler)
      when is_address_type(address_type) and is_message_handler(handler) do
    Storage.put({:address_type_message_handler, address_type}, handler)
  end

  @doc """
  Unsets the default message handler or the handler for an address type.

  Always returns `:ok`.
  """
  @spec unset_message_handler(:default | RoutableAddress.type()) :: :ok

  def unset_message_handler(:default), do: Storage.delete(:default_message_handler)

  def unset_message_handler(address_type) when is_address_type(address_type) do
    Storage.delete({:address_type_message_handler, address_type})
  end

  @doc false
  # Returns a specification to start this module under a supervisor. When this
  # module is added to a supervisor, the supervisor calls child_spec to figure
  # out the specification that should be used.
  #
  # See the "Child specification" section in the `Supervisor` module for more
  # detailed information.
  def child_spec(options) do
    %{id: __MODULE__, type: :worker, start: {__MODULE__, :start_link, [options]}}
  end

  @doc false
  # Starts the router and links it to the current process.
  def start_link(options) do
    return_value = Storage.start_link(options)

    metadata = %{options: options, return_value: return_value}
    Telemetry.emit_event([__MODULE__, :start_link], metadata: metadata)

    return_value
  end
end
