defmodule Ockam.Channel do
  @moduledoc """
  A channel provides end-to-end secure and private communication that is
  safe against eavesdropping, tampering, and forgery of messages en-route.
  """

  use GenStateMachine

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Vault

  require Logger

  # Set the name of the supervisor that will be used to start workers.
  # The supervisor is assumed to be a DynamicSupervisor later in this module.
  @supervisor Ockam.Channel.Supervisor

  @key_establisher Ockam.Channel.XX

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
      address = GenStateMachine.call(pid, :get_address)
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
  def create(options \\ []) when is_list(options) do
    options =
      options
      |> Enum.into(%{})
      |> Map.put_new_lazy(:address, fn -> Router.get_unused_address() end)

    on_start_child = DynamicSupervisor.start_child(@supervisor, {__MODULE__, options})

    with {:ok, pid, worker} <- on_start_child do
      if GenStateMachine.call(pid, :get_role) === :initiator do
        Kernel.send(pid, {:trigger, Map.get(options, :onward_route, [])})
      end

      {:ok, worker}
    end
  end

  @doc false
  def start_link(%{address: nil}), do: {:error, :address_cannot_be_nil}

  def start_link(%{address: address} = options) do
    name = {:via, Router, address}

    with {:ok, pid} <- GenStateMachine.start_link(__MODULE__, options, name: name) do
      {:ok, pid, %__MODULE__{address: address}}
    end
  end

  @impl true
  def init(options) do
    {:ok, plaintext_collecter} = Ockam.Worker.create fn(m) ->
      __MODULE__.send %__MODULE__{address: options.address}, {:plaintext, m}
    end

    options
    |> Map.put_new(:role, :initiator)
    |> Map.put_new(:route_incoming_messages_to, [])
    |> Map.put_new(:plaintext_address, plaintext_collecter.address)
    |> @key_establisher.init
  end

  @impl true
  def handle_event({:call, from}, :get_address, state, %{address: address} = data) do
    {:next_state, state, data, [{:reply, from, address}]}
  end

  @impl true
  def handle_event({:call, from}, :get_plaintext_address, state, %{plaintext_address: address} = data) do
    {:next_state, state, data, [{:reply, from, address}]}
  end

  def handle_event({:call, from}, :get_role, state, %{role: role} = data) do
    {:next_state, state, data, [{:reply, from, role}]}
  end

  def handle_event(:info, event, {:key_establishment, role, s}, data) do
    @key_establisher.handle(event, {:key_establishment, role, s}, data)
  end

  def handle_event(:info, {:plaintext, m}, :data, %{data_state: state} = data) do
    %Message{payload: plaintext} = m
    %{route_to_peer: route_to_peer, vault: vault, encrypt: {encryption_key, nonce}, h: h} = state

    {:ok, ciphertext} = Vault.encrypt(vault, encryption_key, nonce, h, Message.encode(m))
    nonce = nonce + 1

    message = %Message{
      payload: ciphertext,
      onward_route: route_to_peer,
      return_route: [data.address]
    }

    Router.route(message)
    {:next_state, :data, %{data | data_state: %{state | encrypt: {encryption_key, nonce}}}}
  end

  @impl true
  def handle_event(:info, m, :data, %{data_state: state} = data) do
    %Message{payload: ciphertext} = m
    %{vault: vault, decrypt: {decryption_key, nonce}, h: h} = state

    {:ok, plaintext} = Vault.decrypt(vault, decryption_key, nonce, h, ciphertext)
    nonce = nonce + 1

    Logger.debug("Incoming #{inspect(plaintext)}")

    message = Message.decode(plaintext)
    %Message{onward_route: onward, return_route: return} = message

    # if top of onward_route is my address then pop it.
    onward =
      if data.address === List.first(onward) do
        [_head | tail] = onward
        tail
      else
        onward
      end

    onward =
      if data.route_incoming_messages_to === [] do
        onward
      else
        data.route_incoming_messages_to ++ onward
      end

    # if incoming return_route is empty then assume that replies should
    # be sent to my_address
    return = if [] === return, do: [data.plaintext_address], else: return

    message = %{message | onward_route: onward, return_route: return}
    Logger.debug("Incoming #{inspect(message)}")

    Router.route(message)
    {:next_state, :data, %{data | data_state: %{state | decrypt: {decryption_key, nonce}}}}
  end
end
