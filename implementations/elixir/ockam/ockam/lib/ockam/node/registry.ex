defmodule Ockam.Node.Registry do
  @moduledoc false

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

  @spec start_link(any) :: {:error, any} | {:ok, pid}
  @doc false
  def start_link(_options) do
    # TODO: investigte other registry options like listeners and partitions
    Registry.start_link(keys: :unique, name: __MODULE__)
  end

  @spec register_name(any, any) :: :no | :yes
  @doc false
  # This function is used when a process is registered using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  # def register_name(address, pid), do: Registry.register_name({__MODULE__, address}, pid)
  def register_name(address, pid) do
    Registry.register_name({__MODULE__, address}, pid)
  end

  @spec whereis_name(any) :: :undefined | pid
  @doc false
  # This function is used when a process is registered using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  def whereis_name(address), do: Registry.whereis_name({__MODULE__, address})

  @spec unregister_name(any) :: :ok
  @doc false
  # This function is used when a process is registered using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  def unregister_name(address), do: Registry.unregister_name({__MODULE__, address})

  @doc false
  def update_value(address, value), do: Registry.update_value(__MODULE__, address, value)

  @spec addresses(pid) :: [any]

  def addresses(pid), do: Registry.keys(__MODULE__, pid)

  # This function is used when a process is registered using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def send(address, message), do: Registry.send({__MODULE__, address}, message)

  @doc """
  List all registered worker names
  """
  def list_names(), do: Registry.select(__MODULE__, [{{:"$1", :_, :_}, [], [:"$1"]}])

  @spec register(any(), any()) :: :ok | {:error, reason :: any()}
  @doc false
  # This function is used in custom process registration
  #
  # Module should be the worker implementation module
  def register(address, meta) when is_map(meta) or is_nil(meta) do
    case Registry.register(__MODULE__, address, meta) do
      {:ok, _owner} -> :ok
      {:error, reason} -> {:error, reason}
    end
  end

  @spec update_metadata(address :: any(), callback :: (map() -> map())) :: :ok | :error
  @doc false
  # Set worker metadata for the current process
  #
  # This function is called from the worker behaviour
  # Metadata is not set when registering with register_name from `:via` option
  # so this function needs to be called to set it after the process is created,
  # and whenever we want to update it
  def update_metadata(address, callback) do
    case Registry.update_value(__MODULE__, address, callback) do
      :error -> :error
      {_new, _old} -> :ok
    end
  end

  @spec lookup(address :: any()) :: {:ok, pid(), module() | nil} | :error
  def lookup(address) do
    case Registry.lookup(__MODULE__, address) do
      [{pid, nil}] -> {:ok, pid, nil}
      [{pid, meta}] when is_map(meta) -> {:ok, pid, Map.get(meta, :module)}
      [] -> :error
    end
  end

  @spec lookup_meta(address :: any()) :: {:ok, map()} | :error
  def lookup_meta(address) do
    case Registry.lookup(__MODULE__, address) do
      [{_pid, nil}] -> {:ok, %{}}
      [{_pid, meta}] when is_map(meta) -> {:ok, meta}
      [] -> :error
    end
  end

  def select_by_attribute(name, value) do
    Registry.select(__MODULE__, [{{:_, :_, %{attributes: %{name => value}}}, [], [{{:"$_"}}]}])
    |> Enum.map(fn {{addr, {_pid, %{attributes: attributes}}}} -> {addr, attributes} end)
  end
end
