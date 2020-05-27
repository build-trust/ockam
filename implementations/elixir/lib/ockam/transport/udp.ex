defmodule Ockam.Transport.UDP do
  @moduledoc """
  Defines an Ockam UDP Transport
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
  @supervisor Ockam.Transport.UDP.Supervisor

  defstruct [:address]

  @typedoc "The udp transport address type."
  @type address :: Router.address()

  @typedoc "The udp transport type."
  @type t :: %__MODULE__{address: address}

  @doc """
  Sends a message to the given `udp transport`.
  """
  @spec send(t, any) :: any
  def send(%__MODULE__{address: address}, message), do: Router.route(address, message)

  @doc """
  Returns the `pid` of the given `udp transport`.
  """
  @spec whereis(t) :: pid
  def whereis(%__MODULE__{address: address}), do: Router.whereis(address)

  @doc """
  Returns a list of all udp transports currently known to `@supervisor`.
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
  Destroy the given udp transport.
  """
  @spec destroy(t) :: :ok | {:error, :not_found}
  def destroy(%__MODULE__{address: address} = worker) do
    pid = whereis(worker)
    Router.unregister(address)

    DynamicSupervisor.terminate_child(@supervisor, pid)
  end

  @doc """
  Creates a new udp transport.
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

    # tell the router to send outgoing udp messages to this process.
    if Map.get(options, :become_outgoing_udp_router, true) do
      Router.register(:udp, self())
    end

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
    Logger.debug("Incoming #{inspect(incoming)}")

    %{ip: my_ip, port: my_port} = state

    message = Message.decode(packet)
    %Message{onward_route: onward, return_route: return} = message

    # if top of onward_route is my_ip, my_port then pop it.
    onward =
      if {:udp, {my_ip, my_port}} === List.first(onward) do
        [_head | tail] = onward
        tail
      else
        onward
      end

    onward =
      if state.route_incoming_messages_to === [] do
        onward
      else
        [state.route_incoming_messages_to | onward]
      end

    # if incoming return_route is empty then assume that replies should
    # be sent to from_ip, from_port

    return =
      if [] === return do
        [{:udp, {from_ip, from_port}}]
      else
        return
      end

    message = %{message | onward_route: onward, return_route: return}
    Logger.debug("Incoming #{inspect(message)}")

    Router.route(message)
    {:noreply, state}
  rescue
    error ->
      Logger.error("Incoming error: #{inspect({error, incoming, state})}")
      {:noreply, state}
  end

  # handles an outgoing message
  def handle_info({_, %Message{} = outgoing}, state) do
    handle_info(outgoing, state)
  end

  def handle_info(outgoing, state) do
    Logger.debug("Outgoing #{inspect({outgoing, state})}")
    %{ip: my_ip, port: my_port} = state
    %Message{onward_route: onward, return_route: return} = outgoing

    # if top of onward_route is my_ip, my_port then pop it.
    onward =
      if {:udp, {my_ip, my_port}} === List.first(onward) do
        [_head | tail] = onward
        tail
      else
        onward
      end

    # pop top of onward_route and use it as to_ip, to_port.
    [{:udp, {to_ip, to_port}} | _tail] = onward

    # add my_ip, my_port to return_route
    message = %{
      outgoing
      | onward_route: onward,
        return_route: [{:udp, {my_ip, my_port}} | return]
    }

    Logger.debug("Outgoing #{inspect(message)}")

    encoded = Message.encode(message)
    :ok = :gen_udp.send(state.socket, to_ip, to_port, encoded)
    {:noreply, state}
  rescue
    error ->
      Logger.error("Outgoing error: #{inspect({error, outgoing, state})}")
      {:noreply, state}
  end
end
