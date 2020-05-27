defmodule Ockam.Worker do
  @moduledoc """
  A worker is a process that invokes a supplied function whenever it
  receives a new message.

  Workers may be `stateless` or `statful`. A simple stateless worker
  can be created passing an anonymous function to the `create/1` function:

      {:ok, worker} = Ockam.Worker.create fn message ->
        IO.puts(message)
      end
  """

  # use GenServer, makes this module a GenServer.
  #
  # Among other things, it adds the `child_spec/1` function which returns a
  # specification to start this module under a supervisor. When this module is
  # added to a supervisor, the supervisor calls child_spec to figure out the
  # specification that should be used.
  #
  # See the "Child specification" section in the `Supervisor` module for more
  # detailed information.
  #
  # The `@doc` annotation immediately preceding `use GenServer` below
  # is attached to the generated `child_spec/1` function. Since we don't
  # want `child_spec/1` in our Worker module docs, `@doc false` is set here.

  @doc false
  use GenServer

  alias Ockam.Router

  require Logger

  # Set the name of the supervisor that will be used to start workers.
  # The supervisor is assumed to be a DynamicSupervisor later in this module.
  @supervisor Ockam.Worker.Supervisor

  defstruct [:address]

  @typedoc "The worker address type."
  @type address :: Router.address()

  @typedoc "The worker type."
  @type t :: %__MODULE__{address: address}

  @doc """
  Sends a message to the given `worker`.
  """
  @spec send(t, any) :: any
  def send(%__MODULE__{address: address}, message), do: Router.route(address, message)

  @doc """
  Returns the `pid` of the given `worker`.
  """
  @spec whereis(t) :: pid
  def whereis(%__MODULE__{address: address}), do: Router.whereis(address)

  @doc """
  Returns a list of all workers currently known to `Ockam.Worker.Supervisor`.
  """
  @spec list() :: [t]
  def list do
    @supervisor
    |> DynamicSupervisor.which_children()
    |> Enum.reduce([], fn {_, pid, _, _}, workers ->
      address = GenServer.call(pid, :get_address)
      [%__MODULE__{address: address} | workers]
    end)
  end

  @doc """
  Destroy the given worker.
  """
  @spec destroy(t) :: :ok | {:error, :not_found}
  def destroy(%__MODULE__{address: address} = worker) do
    pid = whereis(worker)
    Router.unregister(address)

    DynamicSupervisor.terminate_child(@supervisor, pid)
  end

  @doc """
  Creates a new worker.
  """
  @spec create(any, Keyword.t() | fun) :: {:ok, t} | {:error, term}
  def create(handler_state \\ nil, handler_or_options) do
    options = prepare_options(handler_state, handler_or_options)
    options = Map.put_new_lazy(options, :address, fn -> Router.get_unused_address() end)

    on_start_child = DynamicSupervisor.start_child(@supervisor, {__MODULE__, options})
    with {:ok, _pid, worker} <- on_start_child, do: {:ok, worker}
  end

  # prepare_options/2 is a helper used by create/2 to validate its arguments
  # and prepare an options map that it will send to DynamicSupervisor.start_child
  # which then inturn sends it to __MODULE__.start_link/1

  defp prepare_options(nil, options) when is_list(options), do: Enum.into(options, %{})

  defp prepare_options(nil, handler) when is_function(handler),
    do: %{handler: {:stateless, handler}}

  defp prepare_options(handler_state, handler) when is_function(handler),
    do: %{handler: {:stateful, handler, handler_state}}

  # start_link/1 starts a Worker process linked to the current process.

  @doc false
  def start_link(%{address: nil}), do: {:error, :address_cannot_be_nil}

  def start_link(%{address: address} = options) do
    with {:ok, pid} <- GenServer.start_link(__MODULE__, options, name: {:via, Router, address}) do
      {:ok, pid, %__MODULE__{address: address}}
    end
  end

  @doc false
  @impl true
  def init(options) when is_map(options) do
    with :ok <- check_handler(options) do
      state = options

      Logger.info("Starting new #{__MODULE__} with state - #{inspect(state)}")
      {:ok, state}
    end
  end

  # check_handler/{1,2} validates that the handler is a function and it
  # has the correct arity (number of aruments its accepts)
  #
  # if the worker is stateless the handler should have arity 1
  # if the worker is stateful the handler should have arity 1

  defp check_handler(%{handler: {:stateless, fun}}), do: check_handler(1, fun)
  defp check_handler(%{handler: {:stateful, fun, _}}), do: check_handler(2, fun)

  defp check_handler(expected_arity, fun) when is_function(fun) do
    case :erlang.fun_info(fun)[:arity] === expected_arity do
      true -> :ok
      false -> {:error, {:unexpected_arity, [handler: fun, expected_arity: expected_arity]}}
    end
  end

  @doc false
  @impl true
  def handle_call(:get_address, _from, %{address: address} = state),
    do: {:reply, address, state}

  @doc false
  @impl true

  # handle a message when handler is stateless
  def handle_info(message, %{handler: {:stateless, handler}} = state) do
    apply(handler, [message])
    {:noreply, state}
  end

  # handle a message when handler is stateful
  def handle_info(message, %{handler: {:stateful, handler, handler_state}} = state) do
    case apply(handler, [message, handler_state]) do
      {:ok, new_handler_state} ->
        {:noreply, %{state | handler: {:stateful, handler, new_handler_state}}}

      {:error, error} ->
        {:stop, {:error, error}, handler_state}
    end
  end
end
