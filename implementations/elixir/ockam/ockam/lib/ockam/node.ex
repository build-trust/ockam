defmodule Ockam.Node do
  @moduledoc false

  @doc false
  use Supervisor

  alias Ockam.Address
  alias Ockam.Message
  alias Ockam.Node.Registry
  alias Ockam.Router
  alias Ockam.Telemetry

  # `get_random_unused_address/1` uses this as the length of the new address
  # that will be generated.
  @default_address_length_in_bytes 8

  # Name of the DynamicSupervisor used to supervise processes
  # created with `start_supervised/2`
  @processes_supervisor __MODULE__.ProcessSupervisor

  @doc """
  Returns the process registry for this node.
  """
  def process_registry, do: Registry

  @doc """
  Returns the `pid` of registered address, or `nil`.
  """
  def whereis(address) do
    case Registry.whereis_name(address) do
      :undefined -> nil
      pid -> pid
    end
  end

  def register_address(address) do
    Registry.register_name(address, self())
  end

  @doc """
  Registers the address of a `pid`.
  """
  defdelegate register_address(address, pid), to: Registry, as: :register_name

  @doc """
  Unregisters an address.
  """
  defdelegate unregister_address(address), to: Registry, as: :unregister_name

  @doc """
  Lists all registered addresses
  """
  defdelegate list_addresses(), to: Registry, as: :list_names

  @doc """
  Send a message to the process registered with an address.
  """
  def send(address, %Ockam.Message{} = message) do
    case Registry.whereis_name(address) do
      # dead letters
      :undefined -> :ok
      _pid -> Registry.send(address, message)
    end
  end

  def register_random_address(prefix \\ "", length_in_bytes \\ @default_address_length_in_bytes) do
    address = get_random_unregistered_address(prefix, length_in_bytes)

    case register_address(address) do
      :yes -> {:ok, address}
      ## TODO: recursion limit
      :no -> register_random_address(prefix, length_in_bytes)
    end
  end

  @doc """
  Returns a random address that is currently not registed on the node.
  """
  def get_random_unregistered_address(
        prefix \\ "",
        length_in_bytes \\ @default_address_length_in_bytes
      ) do
    random = length_in_bytes |> :crypto.strong_rand_bytes() |> Base.encode16(case: :lower)
    candidate = prefix <> random

    case whereis(candidate) do
      nil -> candidate
      ## TODO: recursion limit
      _pid -> get_random_unregistered_address(prefix, length_in_bytes)
    end
  end

  @doc false
  def start_supervised(module, options) do
    DynamicSupervisor.start_child(
      @processes_supervisor,
      Supervisor.child_spec({module, options}, restart: :transient)
    )
  end

  @doc false
  def start_link(_init_arg) do
    Supervisor.start_link(__MODULE__, nil, name: __MODULE__)
  end

  @doc false
  def stop(pid) when is_pid(pid) do
    DynamicSupervisor.terminate_child(@processes_supervisor, pid)
  end

  def stop(address) do
    case Registry.whereis_name(address) do
      pid when is_pid(pid) ->
        stop(pid)

      _other ->
        :ok
    end
  end

  @doc false
  @impl true
  def init(nil) do
    with :ok <- Router.set_message_handler(:default, &handle_local_message/1),
         :ok <- Router.set_message_handler(0, &handle_local_message/1) do
      # Specifications of child processes that will be started and supervised.
      #
      # See the "Child specification" section in the `Supervisor` module for more
      # detailed information.
      children = [
        Registry,
        {DynamicSupervisor,
         strategy: :one_for_one, name: @processes_supervisor, max_restarts: 100}
      ]

      # Start a supervisor with the given children. The supervisor will inturn
      # start the given children.
      #
      # The :one_for_all supervision strategy is used, if a child process
      # terminates, all other child processes are terminated and then all child
      # processes (including the terminated one) are restarted.
      #
      # See the "Strategies" section in the `Supervisor` module for more
      # detailed information.
      Supervisor.init(children, strategy: :one_for_all)
    end
  end

  def handle_local_message(%Ockam.Message{} = message) do
    metadata = %{message: message}

    start_time =
      Telemetry.emit_start_event([__MODULE__, :handle_local_message], metadata: metadata)

    return_value =
      case Message.onward_route(message) do
        [] ->
          # Logger.warn("Routing message with no onward_route: #{inspect(message)}")
          :ok

        [first | _rest] ->
          0 = Address.type(first)
          local_address = Address.value(first)
          __MODULE__.send(local_address, message)
      end

    metadata = Map.put(metadata, :return_value, return_value)

    Telemetry.emit_stop_event([__MODULE__, :handle_local_message], start_time, metadata: metadata)

    return_value
  end
end
