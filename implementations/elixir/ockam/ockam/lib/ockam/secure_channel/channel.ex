defmodule Ockam.SecureChannel.Channel do
  @moduledoc false

  ## TODO: make that Ockam.Worker
  use GenStateMachine, callback_mode: [:handle_event_function, :state_enter]

  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Router
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm, as: EncryptedTransport
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX, as: XXKeyEstablishmentProtocol
  alias Ockam.Telemetry
  alias Ockam.Wire

  require Logger

  @doc false
  def peer(_channel), do: :ok

  def established?(channel) do
    channel |> Node.whereis() |> GenStateMachine.call(:established?)
  end

  @doc false
  @spec create(options :: Keyword.t(), timeout :: integer()) ::
          {:ok, address :: Ockam.Address.t()}
          | {:error, {:init_timeout, {pid :: pid(), address :: Ockam.Address.t()}}}
          | {:error, reason :: any()}
  def create(options, timeout \\ 30_000) when is_list(options) do
    ## TODO: why secure channel is not a worker?
    address_prefix = Keyword.get(options, :address_prefix, "")

    options =
      Keyword.put_new_lazy(options, :address, fn ->
        Node.get_random_unregistered_address(address_prefix)
      end)

    case Node.start_supervised(__MODULE__, options) do
      {:ok, pid, address} ->
        with {:ok, _pid, address} <- wait_for_channel(pid, address, timeout) do
          {:ok, address}
        end

      error ->
        error
    end
  end

  def wait_for_channel(pid, address, timeout, interval \\ 100)

  def wait_for_channel(pid, address, timeout, _interval)
      when is_integer(timeout) and timeout < 0 do
    {:error, {:init_channel, :timeout, {pid, address}}}
  end

  def wait_for_channel(pid, address, timeout, interval) when is_integer(timeout) do
    case established?(address) do
      false ->
        :timer.sleep(interval)
        wait_for_channel(pid, address, timeout - interval, interval)

      true ->
        {:ok, pid, address}
    end
  end

  def wait_for_channel(pid, address, :infinity, interval) do
    case established?(address) do
      false ->
        :timer.sleep(interval)
        wait_for_channel(pid, address, :infinity, interval)

      true ->
        {:ok, pid, address}
    end
  end

  @doc false
  def start_link(options) when is_list(options) do
    address = Keyword.get(options, :address, Node.get_random_unregistered_address())
    options = Keyword.put(options, :address, address)

    with {:ok, pid} <- start_link(address, options) do
      {:ok, pid, address}
    end
  end

  defp start_link(address, options) do
    name = {:via, Node.process_registry(), address}
    GenStateMachine.start_link(__MODULE__, options, name: name)
  end

  @doc false
  @impl true
  def init(options) do
    Node.set_address_module(Keyword.fetch!(options, :address), __MODULE__)
    metadata = %{options: options}
    start_time = Telemetry.emit_start_event([__MODULE__, :init], metadata: metadata)

    vault = ensure_vault(options)
    options = Keyword.put(options, :vault, vault)

    with {:ok, data} <- setup_plaintext_address(options, %{}),
         {:ok, data} <- setup_ciphertext_address(options, data),
         {:ok, data} <- setup_vault(options, data),
         {:ok, data} <- setup_peer(options, data),
         {:ok, data} <- setup_initiating_message(options, data),
         {:ok, data} <- setup_callback_route(options, data),
         {:ok, data} <- setup_extra_init_payload(options, data),
         {:ok, data} <- setup_authorization(options, data),
         {:ok, initial, data, next_events} <- setup_key_establishment_protocol(options, data) do
      return_value = {:ok, initial, data, next_events}

      metadata = Map.put(metadata, :return_value, return_value)
      Telemetry.emit_stop_event([__MODULE__, :init], start_time, metadata: metadata)

      return_value
    end
  end

  @doc false
  @impl true
  def handle_event(:enter, _event, {:key_establishment, _role, _role_state}, _data) do
    :keep_state_and_data
  end

  def handle_event(:enter, _event, {:encrypted_transport, :ready}, data) do
    notify_callback_route(data)
    :keep_state_and_data
  end

  def handle_event(event_type, event, state, data) do
    metadata = %{event_type: event_type, event: event, state: state, data: data}
    start_time = Telemetry.emit_start_event([__MODULE__, :handle_event], metadata: metadata)

    return_value = do_handle_event(event_type, event, state, data)

    metadata = Map.put(metadata, :return_value, return_value)
    Telemetry.emit_stop_event([__MODULE__, :handle_event], start_time, metadata: metadata)

    return_value
  end

  defp do_handle_event({:call, from}, :established?, state, data) do
    established = {:encrypted_transport, :ready} === state
    {:next_state, state, data, [{:reply, from, established}]}
  end

  defp do_handle_event(:internal, event, {:key_establishment, _role, _role_state} = state, data) do
    key_establishment_protocol = Map.get(data, :key_establishment_protocol)
    key_establishment_protocol.handle_internal(event, state, data)
  end

  defp do_handle_event(:info, %Ockam.Message{} = message, state, data) do
    ## TODO: make secure channel a worker
    result =
      Ockam.Worker.with_handle_message_metric(__MODULE__, message, data, fn ->
        case is_authorized(message, data) do
          :ok ->
            ## reply is using gen_statem format,
            ## not supported by :with_handle_message_metric
            case handle_message(message, state, data) do
              {:error, reason} -> {:error, reason}
              {:stop, reason, data} -> {:stop, reason, data}
              other -> {:ok, other}
            end

          {:error, reason} ->
            {:error, {:unauthorized, reason}}
        end
      end)

    ## handle_message_result is using gen_statem format,
    ## not supported by :with_handle_message_metric
    case result do
      {:ok, handle_message_result} -> handle_message_result
      other -> other
    end
  end

  defp handle_message(message, {:key_establishment, _role, _role_state} = state, data) do
    key_establishment_protocol = Map.get(data, :key_establishment_protocol)
    key_establishment_protocol.handle_message(message, state, data)
  end

  defp handle_message(message, {:encrypted_transport, :ready} = state, data) do
    first_address = message |> Message.onward_route() |> List.first()

    cond do
      first_address === data.ciphertext_address ->
        case decrypt_and_send_to_router(message, state, data) do
          {:next_state, state, data} ->
            {:next_state, state, data}

          {:error, reason} ->
            Logger.warn("packet dropped: #{inspect(reason)}")
            # Erroneous/Repeated/Malicius packet dropped.
            # decryptor state is not affected
            {:next_state, state, data}
        end

      first_address === data.plaintext_address ->
        encrypt_and_send_to_peer(message, state, data)

      true ->
        {:next_state, state, data}
    end
  end

  defp decode_payload(payload) do
    case :bare.decode(payload, :data) do
      {:ok, encrypted, ""} -> {:ok, encrypted}
      _other -> {:error, {:bad_payload, payload}}
    end
  end

  defp decrypt_and_send_to_router(envelope, state, data) do
    %{encrypted_transport: %{decrypt: transport_state} = encrypted_transport} = data
    payload = Message.payload(envelope)

    ## TODO: optimise double encoding of binaries
    with {:ok, encrypted} <- decode_payload(payload),
         {:ok, decrypted, new_transport_state} <-
           EncryptedTransport.Decryptor.decrypt(<<>>, encrypted, transport_state),
         {:ok, decoded} <- Wire.decode(decrypted, :secure_channel) do
      message =
        decoded
        |> Message.trace(data.plaintext_address)

      Router.route(message)

      {:next_state, state,
       %{data | encrypted_transport: %{encrypted_transport | decrypt: new_transport_state}}}
    end
  end

  defp encrypt_and_send_to_peer(message, state, data) do
    %{encrypted_transport: %{encrypt: transport_state} = encrypted_transport} = data
    forwarded_message = Message.forward(message)

    with {:ok, encoded} <- Wire.encode(forwarded_message),
         {:ok, encrypted, new_transport_state} <-
           EncryptedTransport.Encryptor.encrypt(<<>>, encoded, transport_state) do
      ## TODO: optimise double encoding of binaries
      ## Rust implementation is using implicit encoding,
      ## which encodes binaries even if it's not necessary
      payload = :bare.encode(encrypted, :data)

      envelope = %{
        payload: payload,
        onward_route: data.peer.route,
        return_route: [data.ciphertext_address]
      }

      Router.route(envelope)

      {:next_state, state,
       %{data | encrypted_transport: %{encrypted_transport | encrypt: new_transport_state}}}
    end
  end

  defp is_authorized(message, data) do
    Ockam.Worker.is_authorized(message, data)
  end

  defp notify_callback_route(%{
         encrypted_transport: %{h: h},
         callback_route: callback_route,
         plaintext_address: address
       })
       when is_list(callback_route) do
    Ockam.Router.route(%{payload: h, onward_route: callback_route, return_route: [address]})
  end

  defp notify_callback_route(_data) do
    :ok
  end

  # application facing address is plaintext address
  defp setup_plaintext_address(options, data) do
    case Keyword.get(options, :address) do
      nil -> {:error, {:option_is_nil, :address}}
      plaintext_address -> {:ok, Map.put(data, :plaintext_address, plaintext_address)}
    end
  end

  # network facing address is ciphertext address
  defp setup_ciphertext_address(options, data) do
    ## TODO: use a different prefix?
    address_prefix = Keyword.get(options, :address_prefix, "")
    ciphertext_address = Node.get_random_unregistered_address(address_prefix)

    with :ok <- Node.register_address(ciphertext_address, __MODULE__) do
      {:ok, Map.put(data, :ciphertext_address, ciphertext_address)}
    end
  end

  defp ensure_vault(options) do
    case Keyword.fetch(options, :vault) do
      {:ok, vault} ->
        vault

      :error ->
        case Ockam.Vault.Software.init() do
          {:ok, vault} ->
            vault

          _error ->
            nil
        end
    end
  end

  # sets vault based on - vault option
  defp setup_vault(options, data) do
    case Keyword.get(options, :vault) do
      nil ->
        {:error, {:option_is_nil, :vault}}

      vault ->
        {:ok, Map.put(data, :vault, vault)}
    end
  end

  # sets peer based on - route option
  def setup_peer(options, data) do
    route = Keyword.get(options, :route, [])
    {:ok, Map.put(data, :peer, %{route: route})}
  end

  # sets initiating_message
  defp setup_initiating_message(options, data) do
    case Keyword.get(options, :initiating_message) do
      nil -> {:ok, data}
      initiating_message -> {:ok, Map.put(data, :initiating_message, initiating_message)}
    end
  end

  defp setup_callback_route(options, data) do
    case Keyword.get(options, :callback_route) do
      nil ->
        {:ok, data}

      callback_route when is_list(callback_route) ->
        {:ok, Map.put(data, :callback_route, callback_route)}
    end
  end

  defp setup_extra_init_payload(options, data) do
    case Keyword.get(options, :extra_init_payload) do
      nil -> {:ok, data}
      extra_init_payload -> {:ok, Map.put(data, :extra_init_payload, extra_init_payload)}
    end
  end

  defp setup_authorization(options, data) do
    case Keyword.get(options, :authorization, []) do
      authorization when is_list(authorization) ->
        plaintext_address = Map.fetch!(data, :plaintext_address)
        ciphertext_address = Map.fetch!(data, :ciphertext_address)
        data = Map.put(data, :all_addresses, [plaintext_address, ciphertext_address])
        {:ok, Ockam.Worker.update_authorization_state(data, plaintext_address, authorization)}

      other ->
        {:error, {:authorization_not_list, other}}
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
end
