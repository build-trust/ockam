defmodule Ockam.Transport.UDP.Portal do
  @moduledoc """
  Defines an Ockam UDP Portal
  """

  @doc false
  use GenServer

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @default_ip {127, 0, 0, 1}
  @default_port 4002

  # Set the name of the supervisor that will be used to start workers.
  # The supervisor is assumed to be a DynamicSupervisor later in this module.
  @supervisor Ockam.Transport.UDP.Portal.Supervisor

  defstruct [:address]

  @typedoc "The udp portal address type."
  @type address :: Router.address()

  @typedoc "The udp portal type."
  @type t :: %__MODULE__{address: address}

  @doc """
  Sends a message to the given `udp portal`.
  """
  @spec send(t, any) :: any
  def send(%__MODULE__{address: address}, message), do: Router.route(address, message)

  @doc """
  Returns the `pid` of the given `udp portal`.
  """
  @spec whereis(t) :: pid
  def whereis(%__MODULE__{address: address}), do: Router.whereis(address)

  @doc """
  Returns a list of all udp portals currently known to `@supervisor`.
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
  Destroy the given udp portal.
  """
  @spec destroy(t) :: :ok | {:error, :not_found}
  def destroy(%__MODULE__{address: address} = worker) do
    pid = whereis(worker)
    Router.unregister(address)

    DynamicSupervisor.terminate_child(@supervisor, pid)
  end

  @doc """
  Creates a new udp portal.
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

  @impl true
  def init(options) do
    options = Map.put_new(options, :ip, @default_ip)
    options = Map.put_new(options, :port, @default_port)
    options = Map.put_new(options, :route_incoming_messages_to, [])

    udp_open_options = [:binary, :inet, {:ip, options.ip}, {:active, true}]

    with {:ok, socket} <- :gen_udp.open(options.port, udp_open_options) do
      state = Map.put(options, :socket, socket)

      Logger.info("Starting new #{__MODULE__} with state - #{inspect(state)}")
      {:ok, state}
    end
  end

  @doc false
  @impl true
  def handle_call(:get_address, _from, %{address: address} = state),
    do: {:reply, address, state}

  @doc false
  @impl true

  # handles an incoming UDP packet
  def handle_info({:udp, _socket, from_ip, from_port, packet} = incoming, state) do
    message = %Message{
      payload: packet,
      onward_route: [state.route_incoming_messages_to],
      return_route: [{:udp, {from_ip, from_port}}]
    }

    Logger.debug("Incoming #{inspect(message)}")

    Router.route(message)
    {:noreply, state}
  rescue
    error ->
      Logger.error("Incoming error: #{inspect({error, incoming, state})}")
      {:noreply, state}
  end

  # handles an outgoing message
  def handle_info(outgoing, state) do
    # extract to_ip, to_port & payload
    %Message{onward_route: [{:udp, {to_ip, to_port}} | _], payload: payload} = outgoing

    Logger.debug("Outgoing #{inspect({to_ip, to_port, payload})}")
    :ok = :gen_udp.send(state.socket, to_ip, to_port, payload)
    {:noreply, state}
  rescue
    error ->
      Logger.error("Outgoing error: #{inspect({error, outgoing, state})}")
      {:noreply, state}
  end
end
