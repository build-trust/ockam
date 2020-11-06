defmodule Ockam.SecureChannel.Channel do
  @moduledoc false

  use GenStateMachine

  alias Ockam.Node
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX, as: XXKeyEstablishmentProtocol
  alias Ockam.Telemetry

  @doc false
  def send(channel, message), do: Node.send(channel, message)

  @doc false
  def peer(_channel), do: :ok

  @doc false
  def create(options) when is_list(options) do
    options = Keyword.put_new_lazy(options, :address, &Node.get_random_unregistered_address/0)

    case Node.start_supervised(__MODULE__, options) do
      {:ok, _pid, address} -> {:ok, address}
      error -> error
    end
  end

  @doc false
  def start_link(options) when is_list(options) do
    with {:ok, address} <- get_from_options(:address, options),
         {:ok, pid} <- start(address, options) do
      {:ok, pid, address}
    end
  end

  defp start(address, options) do
    name = {:via, Node.process_registry(), address}
    GenStateMachine.start_link(__MODULE__, options, name: name)
  end

  @doc false
  @impl true
  def init(options) do
    metadata = %{options: options}
    start_time = Telemetry.emit_start_event([__MODULE__, :init], metadata: metadata)

    with {:ok, data} <- setup_plaintext_address(options, %{}),
         {:ok, data} <- setup_ciphertext_address(options, data),
         {:ok, data} <- setup_route_to_peer(options, data),
         {:ok, data} <- setup_initiating_message(options, data),
         {:ok, initial_state, data} <- setup_key_establishment_protocol(options, data) do
      return_value = {:ok, initial_state, data}

      metadata = Map.put(metadata, :return_value, return_value)
      Telemetry.emit_stop_event([__MODULE__, :init], start_time, metadata: metadata)

      return_value
    end
  end

  @doc false
  @impl true
  def handle_event(event_type, event, state, data) do
    metadata = %{event_type: event_type, event: event, state: state, data: data}
    start_time = Telemetry.emit_start_event([__MODULE__, :handle_event], metadata: metadata)

    return_value = handle_message(event_type, event, state, data)

    metadata = Map.put(metadata, :return_value, return_value)
    Telemetry.emit_stop_event([__MODULE__, :handle_event], start_time, metadata: metadata)

    return_value
  end

  defp handle_message(:info, event, {:key_establishment, _role, _role_state} = state, data) do
    key_establishment_protocol = Map.get(data, :key_establishment_protocol)
    key_establishment_protocol.handle_message(event, state, data)
  end

  # TODO: should we catah all?
  # defp handle_message(_event_type, _event, _state, _data)

  # application facing address is plaintext address
  defp setup_plaintext_address(options, data) do
    case Keyword.get(options, :address) do
      nil -> {:error, {:option_is_nil, :address}}
      plaintext_address -> {:ok, Map.put(data, :plaintext_address, plaintext_address)}
    end
  end

  # network facing address is ciphertext address
  defp setup_ciphertext_address(_options, data) do
    ciphertext_address = Node.get_random_unregistered_address()

    with :yes <- Node.register_address(ciphertext_address, self()) do
      {:ok, Map.put(data, :ciphertext_address, ciphertext_address)}
    end
  end

  # sets route to peer based on - route option
  def setup_route_to_peer(options, data) do
    case Keyword.get(options, :route) do
      nil -> {:ok, data}
      route -> {:ok, Map.put(data, :route_to_peer, route)}
    end
  end

  # sets initiating_message
  defp setup_initiating_message(options, data) do
    case Keyword.get(options, :initiating_message) do
      nil -> {:ok, data}
      initiating_message -> {:ok, Map.put(data, :initiating_message, initiating_message)}
    end
  end

  # sets a key establishment protocol and calls its setup
  defp setup_key_establishment_protocol(options, data) do
    case Keyword.get(options, :key_establishment_protocol, XXKeyEstablishmentProtocol) do
      XXKeyEstablishmentProtocol ->
        data = Map.put(data, :key_establishment_protocol, XXKeyEstablishmentProtocol)
        data.key_establishment_protocol.setup(options, data)

      unexpected_protocol ->
        {:error, {:unexpected_key_establishment_protocol, unexpected_protocol}}
    end
  end

  @doc false
  defp get_from_options(key, options) do
    case Keyword.get(options, key) do
      nil -> {:error, {:option_is_nil, key}}
      value -> {:ok, value}
    end
  end
end
