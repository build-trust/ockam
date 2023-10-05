defmodule Ockam.Node do
  @moduledoc false

  @doc false
  use Supervisor

  alias Ockam.Address
  alias Ockam.Message
  alias Ockam.Node.Registry
  alias Ockam.Router
  alias Ockam.Telemetry

  require Logger

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

  @spec register_address(any(), module()) :: :ok | {:error, any()}
  @doc """
  Registers the address of the current process with optional module name
  """
  def register_address(address, module \\ nil) do
    self = self()

    case Registry.register(address, module) do
      :ok -> :ok
      {:error, {:already_registered, ^self}} -> :ok
      error -> error
    end
  end

  @spec set_address_module(any(), module()) :: :ok | :error
  @doc """
  Sets module name for already registered process
  """
  def set_address_module(address, module) do
    Registry.set_module(address, module)
  end

  @spec get_address_module(any()) :: {:ok, module()} | :error
  @doc """
  Gets registered worker module by address
  """
  def get_address_module(address) do
    with {:ok, _pid, module} <- Registry.lookup(address) do
      {:ok, module}
    end
  end

  @spec unregister_address(any()) :: :ok
  @doc """
  Unregisters an address.
  """
  defdelegate unregister_address(address), to: Registry, as: :unregister_name

  @spec list_addresses() :: [address :: any()]
  @doc """
  Lists all registered addresses
  """
  defdelegate list_addresses(), to: Registry, as: :list_names

  @spec list_workers() :: [{address :: any(), pid(), module()}]
  @doc """
  Lists all workers with their primary address, worker pid and module
  """
  ## TODO: currently taking just one random address per pid, make sure it's primary
  def list_workers() do
    list_addresses()
    |> Enum.flat_map(fn address ->
      case Registry.lookup(address) do
        {:ok, pid, module} -> [{address, pid, module}]
        :error -> []
      end
    end)
    |> Enum.uniq_by(fn {_address, pid, _module} -> pid end)
  end

  @doc """
  List all registered addresses for a worker
  """
  defdelegate list_addresses(pid), to: Registry, as: :addresses

  @doc """
  Send a message to the process registered with an address.
  """
  def send(address, %Ockam.Message{} = message) do
    case Registry.whereis_name(address) do
      # dead letters
      :undefined ->
        report_message(:unsent, address, message)
        :ok

      _pid ->
        Registry.send(address, message)
        report_message(:sent, address, message)
        :ok
    end
  end

  @spec report_message(:sent | :unsent, any(), Ockam.Message.t()) :: :ok
  def report_message(sent_status, address, message) do
    from = Enum.at(Message.return_route(message), 0)

    metadata = %{from: from, to: address, message: message}

    Telemetry.emit_event([__MODULE__, :message, sent_status],
      measurements: %{count: 1},
      metadata: metadata
    )
  end

  @spec register_random_address(prefix :: String.t(), module(), length_in_bytes :: integer()) ::
          {:ok, address :: any()} | {:error, any()}
  @doc """
  Registers random address of certain length using set prefix and module name
  """
  ## TODO: make address actually fit into length in bytes
  def register_random_address(
        prefix \\ "",
        module \\ nil,
        length_in_bytes \\ @default_address_length_in_bytes
      ) do
    address = get_random_unregistered_address(prefix, length_in_bytes)

    case register_address(address, module) do
      :ok ->
        {:ok, address}

      {:error, {:already_registered, _pid}} ->
        register_random_address(prefix, module, length_in_bytes)

      {:error, reason} ->
        {:error, reason}
    end
  end

  @spec get_random_unregistered_address(prefix :: String.t(), length_in_bytes :: integer()) ::
          binary()
  @doc """
  Returns a random address that is currently not registered on the node.
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
    restart_type = Keyword.get(options, :restart_type, :transient)

    DynamicSupervisor.start_child(
      @processes_supervisor,
      Supervisor.child_spec({module, options}, restart: restart_type)
    )
  end

  def start_link(_init_arg) do
    Supervisor.start_link(__MODULE__, nil, name: __MODULE__)
  end

  def stop(pid) when is_pid(pid) do
    GenServer.stop(pid)
  catch
    ## It's OK if the worker is already stopped
    :exit, {:noproc, _} ->
      :ok
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
    case Message.onward_route(message) do
      [] ->
        report_message(:unsent, nil, message)
        # Logger.warning("Routing message with no onward_route: #{inspect(message)}")
        :ok

      [first | _rest] ->
        0 = Address.type(first)
        local_address = Address.value(first)
        __MODULE__.send(local_address, message)
    end
  end
end
