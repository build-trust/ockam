defmodule Ockam.Node do
  @moduledoc false

  # `get_unused_address/1` uses this as the length of the new address
  # that will be generated.
  @default_address_length_in_bytes 4

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

  # Starts the node and links it to the current process.
  @doc false
  def start_link(_options) do
    # TODO: investigte other registry options like listeners and partitions
    Registry.start_link(keys: :unique, name: __MODULE__)
  end

  @doc """
  Registers the address of a `pid`.
  """
  def register(address, pid), do: register_name(address, pid)

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def register_name(address, pid), do: Registry.register_name({__MODULE__, address}, pid)

  @doc """
  Returns the `pid` of registered address, or `nil` is
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

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def send(address, message), do: Registry.send({__MODULE__, address}, message)

  @doc """
  Returns a random address that is currently not registed with the node.
  """
  def get_unused_address(length_in_bytes \\ @default_address_length_in_bytes) do
    cadidate = :crypto.strong_rand_bytes(length_in_bytes)

    case whereis(cadidate) do
      nil -> cadidate
      _pid -> get_unused_address(length_in_bytes)
    end
  end
end
