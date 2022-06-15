defmodule Ockam.Router do
  @moduledoc """
  Routes messages.

  The default message handler is invoked for a message if the onward route of
  the message is empty or if the first address in the onward route has a type
  for which there is no address type specific handler.
  """

  import Ockam.Address, only: [is_address_type: 1]
  import Ockam.Router.MessageHandler, only: [is_message_handler: 1]

  alias Ockam.Address
  alias Ockam.Message
  alias Ockam.Router.MessageHandler
  alias Ockam.Router.Storage

  alias Ockam.Telemetry

  @type message() ::
          Message.t()
          | %{payload: binary(), onward_route: [Address.t()], return_route: [Address.t()]}

  @dialyzer {:nowarn_function, raise_invalid_message: 1}

  @doc """
  Routes the given message.

  Message can be Ockam.Message, or a map with payload, onward_route and optional return_route
  """
  @spec route(message()) :: :ok | {:error, reason :: any()}

  def route(%Ockam.Message{payload: pl, onward_route: o_r, return_route: r_r} = message)
      when is_binary(pl) and is_list(o_r) and is_list(r_r) do
    message = prepare_local_metadata(message)

    metadata = %{message: message}
    start_time = Telemetry.emit_start_event([__MODULE__, :route], metadata: metadata)

    return_value = pick_and_invoke_handler(message)

    metadata = Map.put(metadata, :return_value, return_value)
    Telemetry.emit_stop_event([__MODULE__, :route], start_time, metadata: metadata)

    return_value
  end

  def route(%{payload: pl, onward_route: o_r} = message) when is_binary(pl) and is_list(o_r) do
    return_route = Map.get(message, :return_route, [])

    case is_list(return_route) do
      true ->
        route(struct(Ockam.Message, message))

      false ->
        raise_invalid_message(message)
    end
  end

  def route(message) do
    raise_invalid_message(message)
  end

  @doc """
  Routes a message with given payload, onward_route and return_route
  """
  def route(payload, onward_route, return_route \\ [], local_metadata \\ %{}) do
    route(%Message{
      onward_route: onward_route,
      return_route: return_route,
      payload: payload,
      local_metadata: local_metadata
    })
  end

  defp prepare_local_metadata(message) do
    metadata =
      case Message.local_metadata(message) do
        %{source: :channel, channel: _channel} = md ->
          md

        ## Make sure metadata with channel has source: channel
        %{channel: _channel} = md ->
          Map.put(md, :source, :channel)

        %{source: _source} = md ->
          md

        ## If there is no source or channel - message is local
        %{} = md ->
          Map.put(md, :source, :local)

        nil ->
          %{source: :local}
      end

    Message.set_local_metadata(message, metadata)
  end

  def raise_invalid_message(message) do
    raise "Cannot route invalid message: #{inspect(message)}"
  end

  defp pick_and_invoke_handler(message) do
    first_address = message |> Message.onward_route() |> List.first()
    handler_type = if first_address, do: Address.type(first_address), else: :default

    case get_message_handler(handler_type) do
      nil -> {:error, {:handler_not_set, handler_type, message}}
      {:error, reason} -> {:error, reason}
      handler -> invoke_handler(handler, message)
    end
  end

  defp invoke_handler(handler, message) do
    case apply_handler(handler, message) do
      {:error, error} -> {:error, {:handler_error, error, message, handler}}
      ## TODO: require and match :ok result
      _anything_else -> :ok
    end
  end

  defp apply_handler(handler, message) when is_function(handler, 1) do
    handler.(message)
  end

  defp apply_handler({m, f, a}, message) when is_atom(m) and is_atom(f) and is_list(a) do
    apply(m, f, [message | a])
  end

  @doc """
  Returns the the handler for an address type or the default message handler.

  Returns `nil` if no message handler is set for the provided address type and
  there is also no `:default` handler set.
  """
  @spec get_message_handler(:default | Address.type()) ::
          MessageHandler.t() | nil | {:error, reason :: any()}

  def get_message_handler(:default), do: Storage.get(:default_message_handler)

  def get_message_handler(0), do: Storage.get(:default_message_handler)

  def get_message_handler(address_type) when Address.is_address_type(address_type) do
    case Storage.get({:address_type_message_handler, address_type}) do
      nil -> nil
      handler -> handler
    end
  end

  @doc """
  Sets the default message handler or the handler for an address type.

  Returns `:ok` if the handler is successfully set.
  """
  @spec set_message_handler(:default | Address.type(), MessageHandler.t()) :: :ok

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
  @spec unset_message_handler(:default | Address.type()) :: :ok

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
