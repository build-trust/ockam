defmodule Ockam.Routing do
  @moduledoc """
  Implements the Ockam Routing Protocol.
  """

  alias Ockam.Routing.Address
  alias Ockam.Routing.Message

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

  # Starts the router and links it to the current process.
  @doc false
  def start_link(_options) do
    # TODO: investigte other registry options like listeners and partitions
    Registry.start_link(keys: :unique, name: __MODULE__)
  end

  @doc """
  Returns the `Ockam.Routing.Address.address_type` that should be used for
  registering local addresses.
  """
  def local_address_type, do: 0

  @doc """
  Returns the `Ockam.Routing.Address` that will be send any message that
  have an empty onward_route.
  """
  def default_address, do: {local_address_type(), 0}

  @doc """
  Registers an address for a process.
  """
  def register(address, process_id) do
    register_name(address, process_id)
  end

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def register_name(address, process_id) do
    Registry.register_name({__MODULE__, address}, process_id)
  end

  @doc """
  Returns the `pid` of registered address, or `nil`
  """
  def whereis(address) do
    case whereis_name(address) do
      :undefined -> nil
      pid -> pid
    end
  end

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def whereis_name(address), do: Registry.whereis_name({__MODULE__, address})

  @doc """
  Unregisters an address.
  """
  def unregister(address), do: unregister_name(address)

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def unregister_name(address), do: Registry.unregister_name({__MODULE__, address})

  @doc """
  Route a message.
  """
  @spec route(Message.t()) :: :ok | {:error, term()}
  def route(message) do
    onward_route = Message.onward_route(message)
    route(onward_route, message)
  end

  # if onward_route is empty, route to default address
  defp route([], message) do
    Registry.send({__MODULE__, default_address()}, message)
  end

  # if onward_route is not empty, route to the first address in the onward_route
  defp route([address | _t], message) do
    address_type = Address.type(address)
    address_value = Address.value(address)
    Registry.send({__MODULE__, {address_type, address_value}}, message)
  end
end
