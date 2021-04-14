defmodule Ockam.Node do
  @moduledoc false

  @doc false
  use Supervisor

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

  @ping <<0>>
  @pong <<1>>

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

  @doc """
  Registers the address of a `pid`.
  """
  defdelegate register_address(address, pid), to: Registry, as: :register_name

  @doc """
  Unregisters an address.
  """
  defdelegate unregister_address(address), to: Registry, as: :unregister_name

  @doc """
  Send a message to the process registered with an address.
  """
  def send(address, message) do
    case Registry.whereis_name(address) do
      # dead letters
      :undefined -> :ok
      _pid -> Registry.send(address, message)
    end
  end

  @doc """
  Returns a random address that is currently not registed on the node.
  """
  def get_random_unregistered_address(length_in_bytes \\ @default_address_length_in_bytes) do
    candidate = length_in_bytes |> :crypto.strong_rand_bytes() |> Base.encode16(case: :lower)

    case whereis(candidate) do
      nil -> candidate
      _pid -> get_random_unregistered_address(length_in_bytes)
    end
  end

  @doc false
  def start_supervised(module, options) do
    DynamicSupervisor.start_child(@processes_supervisor, {module, options})
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
    with :ok <- Router.set_message_handler(:default, &handle_routed_message/1),
         :ok <- Router.set_message_handler(0, &handle_routed_message/1) do
      # Specifications of child processes that will be started and supervised.
      #
      # See the "Child specification" section in the `Supervisor` module for more
      # detailed information.
      children = [
        Registry,
        {DynamicSupervisor, strategy: :one_for_one, name: @processes_supervisor}
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

  def handle_routed_message(message) do
    metadata = %{message: message}

    start_time =
      Telemetry.emit_start_event([__MODULE__, :handle_routed_message], metadata: metadata)

    return_value = route_message(message)

    metadata = Map.put(metadata, :return_value, return_value)

    Telemetry.emit_stop_event([__MODULE__, :handle_routed_message], start_time, metadata: metadata)

    return_value
  end

  def route_message(message) do
    onward_route = Message.onward_route(message)

    case onward_route do
      [] -> handle_control_message(message)
      [{0, <<_::8, _::8, rest::binary>>} | _rest] -> __MODULE__.send(rest, message)
      [first | _rest] -> __MODULE__.send(first, message)
      unexpected_onward_route -> {:error, {:unexpected_onward_route, unexpected_onward_route}}
    end
  end

  def handle_control_message(message) do
    return_route = Message.return_route(message)
    payload = Message.payload(message)

    case payload do
      @ping ->
        reply = %{payload: @pong, onward_route: return_route}
        Router.route(reply)

      unexpected_payload ->
        {:error, {:unexpected_control_instruction, unexpected_payload, message}}
    end
  end
end
