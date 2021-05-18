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
  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  def register_name(address, pid), do: Registry.register_name({__MODULE__, address}, pid)

  @spec whereis_name(any) :: :undefined | pid
  @doc false
  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  def whereis_name(address), do: Registry.whereis_name({__MODULE__, address})

  @spec unregister_name(any) :: :ok
  @doc false
  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  def unregister_name(address), do: Registry.unregister_name({__MODULE__, address})

  # This function is used when a process is registed using the `:via` option.
  #
  # The Gen* modules expect this function to be exported.
  # See the "Name registration" section of the `GenServer` module.
  @doc false
  def send(address, message), do: Registry.send({__MODULE__, address}, message)

  @doc """
  List all registered worker names
  """
  def list_names(), do: Registry.select(__MODULE__, [{{:"$1", :_, :_}, [], [:"$1"]}])
end
