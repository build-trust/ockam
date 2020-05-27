defmodule Ockam.Stream do
  @moduledoc """
  Defines an Ockam Stream
  """

  @doc false
  use GenServer

  alias Ockam.Router

  require Logger

  # Set the name of the supervisor that will be used to start workers.
  # The supervisor is assumed to be a DynamicSupervisor later in this module.
  @supervisor Ockam.Stream.Supervisor

  defstruct [:address]

  @typedoc "The stream address type."
  @type address :: Router.address()

  @typedoc "The stream type."
  @type t :: %__MODULE__{address: address}

  @doc """
  Attaches a consumer with the given `stream`.
  """
  def attach_consumer(%__MODULE__{address: address}, consumer) when is_binary(consumer) do
    Router.whereis(address) |> GenServer.call({:attach_consumer, consumer})
  end

  @doc """
  Sends a message to the given `stream`.
  """
  @spec send(t, any) :: any
  def send(%__MODULE__{address: address}, message), do: Router.route(address, message)

  @doc """
  Returns the `pid` of the given `stream`.
  """
  @spec whereis(t) :: pid
  def whereis(%__MODULE__{address: address}), do: Router.whereis(address)

  @doc """
  Returns a list of all streams currently known to `@supervisor`.
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
  Destroy the given stream.
  """
  @spec destroy(t) :: :ok | {:error, :not_found}
  def destroy(%__MODULE__{address: address} = worker) do
    pid = whereis(worker)
    Router.unregister(address)

    DynamicSupervisor.terminate_child(@supervisor, pid)
  end

  @doc """
  Creates a new stream.
  """
  @spec create(Keyword.t()) :: {:ok, t} | {:error, term}
  def create(options \\ []) when is_list(options) do
    options = Enum.into(options, %{})
    options = Map.put_new_lazy(options, :address, fn -> Router.get_unused_address() end)

    on_start_child = DynamicSupervisor.start_child(@supervisor, {__MODULE__, options})
    with {:ok, _pid, worker} <- on_start_child, do: {:ok, worker}
  end

  @doc false
  def start_link(%{address: nil}), do: {:error, :address_cannot_be_nil}

  def start_link(%{address: address} = options) do
    with {:ok, pid} <- GenServer.start_link(__MODULE__, options, name: {:via, Router, address}) do
      {:ok, pid, %__MODULE__{address: address}}
    end
  end

  @doc false
  @impl true
  def init(%{address: address}) do
    {:ok, %{address: address, consumers: [], messages: []}}
  end

  @doc false
  @impl true
  def handle_call(:get_address, _from, %{address: address} = state),
    do: {:reply, address, state}

  @doc false
  @impl true
  def handle_call({:attach_consumer, consumer}, _from, %{consumers: consumers} = state) do
    {:reply, :ok, %{state | consumers: [consumer | consumers]}}
  end

  @doc false
  @impl true
  def handle_info(message, %{consumers: consumers, messages: messages} = state) do
    Enum.each(consumers, fn consumer -> Router.route(consumer, message) end)
    {:noreply, %{state | messages: [message | messages]}}
  end
end
