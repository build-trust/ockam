defmodule Ockam.Identity.SecureChannel do
  @moduledoc """
  Functions to start identity secure channel and listener
  """

  alias Ockam.Identity

  alias Ockam.Message

  alias Ockam.Session.Pluggable.Initiator
  alias Ockam.Session.Pluggable.Responder

  alias Ockam.Session.Spawner

  require Logger

  @doc """
  Start an identity secure channel listener.

  Usage:
  {:ok, alice, alice_id} = Ockam.Identity.create()
  {:ok, vault} = Ockam.Vault.Software.init()
  create_listener(
    identity: alice,
    encryption_options: [vault: vault],
    address: "alice_listener",
    trust_policies: [{:cached_identity, [Ockam.Identity.TrustPolicy.KnownIdentitiesEts]}])
  """
  def create_listener(options) do
    spawner_options = spawner_options(options)
    Spawner.create(spawner_options)
  end

  @doc """
  Child spec to create listeners
  """
  def listener_child_spec(args) do
    spawner_options = spawner_options(args)

    %{
      id: __MODULE__,
      start: {Spawner, :start_link, [spawner_options]}
    }
  end

  defp spawner_options(options) do
    listener_keys = [:address, :inner_address, :restart_type]
    handshake_options = Keyword.drop(options, listener_keys)

    responder_options = [
      address_prefix: "ISC_R_",
      worker_mod: Ockam.Identity.SecureChannel.Data,
      handshake: Ockam.Identity.SecureChannel.Handshake,
      handshake_options: handshake_options,
      ## TODO: probably all spawners should do that
      restart_type: :temporary
    ]

    Keyword.take(options, listener_keys)
    |> Keyword.merge(
      worker_mod: Responder,
      worker_options: responder_options,
      spawner_setup: &spawner_setup/2
    )
  end

  def spawner_setup(options, state) do
    worker_options = Keyword.fetch!(options, :worker_options)
    handshake_options = Keyword.fetch!(worker_options, :handshake_options)

    identity =
      case Keyword.fetch!(handshake_options, :identity) do
        :dynamic ->
          identity_module = Keyword.fetch!(handshake_options, :identity_module)
          {:ok, new_identity, _id} = Identity.create(identity_module)
          new_identity

        other ->
          other
      end

    new_handshake_options = Keyword.put(handshake_options, :identity, identity)
    new_worker_options = Keyword.put(worker_options, :handshake_options, new_handshake_options)
    {Keyword.put(options, :worker_options, new_worker_options), state}
  end

  @doc """
  Start an identity secure channel.

  Usage:
  {:ok, bob, bob_id} = Ockam.Identity.create()
  {:ok, vault} = Ockam.Vault.Software.init()
  create_channel(
    identity: bob,
    encryption_options: [vault: vault],
    address: "bob_channel",
    route: route_to_listener,
    trust_policies: [{:cached_identity, [Ockam.Identity.TrustPolicy.KnownIdentitiesEts]}])

  By default the function waits for channel session to be established for 30 seconds.
  You can specify a different timeout as a second argument:

  `create_channel(options, timeout)`

  Timeout can be integer or :infinity

  If the session is not established within timeout,
  it will return `{:error, {:timeout, worker}}`
  """
  def create_channel(options, timeout \\ 30_000) do
    init_route = Keyword.fetch!(options, :route)

    encryption_options =
      case Keyword.fetch(options, :encryption_options) do
        {:ok, encryption_options} ->
          encryption_options

        :error ->
          {:ok, vault} = Ockam.Vault.Software.init()
          [vault: vault]
      end

    identity =
      case Keyword.get(options, :identity) do
        nil ->
          module = Keyword.get(options, :identity_module, Identity.default_implementation())
          {:ok, identity, _id} = Identity.create(module)
          identity

        identity ->
          identity
      end

    options = Keyword.merge(options, identity: identity, encryption_options: encryption_options)

    initiator_options = [
      address_prefix: "ISC_I_",
      address: Keyword.get(options, :address),
      worker_mod: Ockam.Identity.SecureChannel.Data,
      init_route: init_route,
      handshake: Ockam.Identity.SecureChannel.Handshake,
      handshake_options: options
    ]

    Initiator.create_and_wait(initiator_options, 100, timeout)
  end
end

defmodule Ockam.Identity.SecureChannel.Handshake do
  @moduledoc """
  Identity secure channel handshake stage

  Implements `Ockam.Session.Handshake`

  Options:
  - identity - own identity of the channel
  - trust_policies - trust policies to check remote contact identity, see `Ockam.Identity.TrustPolicy`
  - encryption_options - options for encryption channel, see `Ockam.SecureChannel.Channel`
  """

  @behaviour Ockam.Session.Handshake

  alias Ockam.Identity
  # alias Ockam.Identity.Stub, as: Identity
  alias Ockam.Identity.SecureChannel.IdentityChannelMessage
  alias Ockam.Identity.TrustPolicy

  alias Ockam.Message
  alias Ockam.SecureChannel.Channel, as: SecureChannel
  alias Ockam.SecureChannel.InitHandshake

  require Logger

  @key_exchange_timeout 20_000

  @impl true
  def init(handshake_options, handshake_state) do
    identity = Keyword.fetch!(handshake_options, :identity)
    trust_policies = Keyword.get(handshake_options, :trust_policies, [])

    init_route = Map.fetch!(handshake_state, :init_route)
    handshake_address = Map.fetch!(handshake_state, :handshake_address)

    {:ok, extra_payload} = Ockam.Wire.encode_address(handshake_address)

    encryption_options =
      Keyword.get(handshake_options, :encryption_options, [])
      |> Keyword.merge(
        role: :initiator,
        route: init_route,
        extra_init_payload: extra_payload,
        callback_route: [handshake_address]
      )

    with {:ok, _pid, enc_channel} <- SecureChannel.start_link(encryption_options),
         {:ok, auth_hash} <- wait_for_key_exchange(enc_channel, handshake_address) do
      new_handshake_state =
        Map.merge(handshake_state, %{
          encryption_channel_options: encryption_options,
          encryption_channel: enc_channel,
          auth_hash: auth_hash,
          identity: identity,
          trust_policies: trust_policies,
          authorization: [:from_secure_channel, {:from_addresses, [:message, [enc_channel]]}]
        })

      {:next, new_handshake_state}
    end
  end

  @impl true
  def handle_initiator(handshake_options, message, state) do
    %{identity: identity, auth_hash: auth_hash, encryption_channel: enc_channel} = state

    payload = Message.payload(message)

    with {:ok, %IdentityChannelMessage.Request{contact: contact_data, proof: proof}} <-
           IdentityChannelMessage.decode(payload),
         {:ok, contact, contact_id} <- Identity.validate_data(identity, contact_data),
         :ok <- Identity.verify_signature(contact, proof, auth_hash),
         :ok <- check_trust(contact, contact_id, state) do
      {:ok, peer_address} = get_peer_address(message)

      state =
        Map.merge(state, %{
          peer_address: peer_address,
          identity: identity
        })

      additional_metadata = Keyword.get(handshake_options, :additional_metadata, %{})

      data_options = [
        peer_address: peer_address,
        encryption_channel: enc_channel,
        identity: identity,
        contact_id: contact_id,
        additional_metadata: additional_metadata
      ]

      {:ready, identity_handshake(IdentityChannelMessage.Response, state), data_options, state}
    end
  end

  ## TODO: stop responders if handshake is not done in limited time
  ## if initiator fails to handshake, responder will be left hanging
  @impl true
  def handle_responder(handshake_options, message, handshake_state) do
    ## TODO: maybe we need some separate handle_init_message for responder?
    ## Currently using a flag in handshake_state ot distinguish between
    ## init message and handshake message
    case Map.get(handshake_state, :expected_message, :init) do
      :init ->
        handle_responder_init(handshake_options, message, handshake_state)

      :handshake ->
        handle_responder_handshake(handshake_options, message, handshake_state)
    end
  end

  def handle_responder_init(handshake_options, message, handshake_state) do
    identity = Keyword.fetch!(handshake_options, :identity)
    trust_policies = Keyword.get(handshake_options, :trust_policies, [])

    init_handshake_payload = Message.payload(message)

    with {:ok, %{handshake: handshake, extra_payload: extra_payload}} <-
           InitHandshake.decode(init_handshake_payload),
         {:ok, peer_address} <- Ockam.Wire.decode_address(extra_payload),
         {:ok, encryption_init} <- make_responder_init_message(message, handshake) do
      handshake_address = Map.get(handshake_state, :handshake_address)

      encryption_options =
        Keyword.get(handshake_options, :encryption_options, [])
        |> Keyword.merge(
          role: :responder,
          initiating_message: encryption_init,
          callback_route: [handshake_address]
        )

      with {:ok, _pid, enc_channel} <- SecureChannel.start_link(encryption_options),
           {:ok, auth_hash} <- wait_for_key_exchange(enc_channel, handshake_address) do
        handshake_state =
          Map.merge(handshake_state, %{
            encryption_channel_options: encryption_options,
            encryption_channel: enc_channel,
            peer_address: peer_address,
            auth_hash: auth_hash,
            expected_message: :handshake,
            identity: identity,
            trust_policies: trust_policies,
            authorization: [:from_secure_channel, {:from_addresses, [:message, [enc_channel]]}]
          })

        {:next, identity_handshake(IdentityChannelMessage.Request, handshake_state),
         handshake_state}
      end
    end
  end

  def handle_responder_handshake(handshake_options, message, state) do
    %{identity: identity, auth_hash: auth_hash} = state
    payload = Message.payload(message)

    with {:ok, %IdentityChannelMessage.Response{contact: contact_data, proof: proof}} <-
           IdentityChannelMessage.decode(payload),
         {:ok, contact, contact_id} <- Identity.validate_data(identity, contact_data),
         :ok <- Identity.verify_signature(contact, proof, auth_hash),
         :ok <- check_trust(contact, contact_id, state),
         {:ok, peer_address} <- get_peer_address(message) do
      enc_channel = Map.get(state, :encryption_channel)

      additional_metadata = Keyword.get(handshake_options, :additional_metadata, %{})

      data_options = [
        peer_address: peer_address,
        encryption_channel: enc_channel,
        identity: identity,
        contact_id: contact_id,
        additional_metadata: additional_metadata
      ]

      state =
        Map.merge(state, %{
          peer_address: peer_address,
          identity: identity
        })

      {:ready, data_options, state}
    end
  end

  defp identity_handshake(type, state) do
    %{
      peer_address: peer_address,
      encryption_channel: encryption_channel,
      identity: identity,
      auth_hash: auth_hash,
      handshake_address: handshake_address
    } = state

    contact_data = Identity.get_data(identity)
    {:ok, proof} = Identity.create_signature(identity, auth_hash)

    payload =
      IdentityChannelMessage.encode(
        struct(type, %{
          contact: contact_data,
          proof: proof
        })
      )

    %Message{
      payload: payload,
      onward_route: [encryption_channel, peer_address],
      return_route: [handshake_address]
    }
  end

  defp check_trust(contact, contact_id, state) do
    policies = Map.get(state, :trust_policies, [])
    identity = Map.fetch!(state, :identity)

    with {:ok, identity_id} <- Identity.validate_identity_change_history(identity) do
      TrustPolicy.from_config(policies, %{id: identity_id, identity: identity}, %{
        id: contact_id,
        identity: contact
      })
    end
  end

  defp wait_for_key_exchange(enc_channel, inner_address, timeout \\ @key_exchange_timeout) do
    receive do
      %Message{payload: auth_hash, onward_route: [^inner_address], return_route: [^enc_channel]} ->
        {:ok, auth_hash}
    after
      timeout ->
        {:error, :key_exchange_timeout}
    end
  end

  defp get_peer_address(message) do
    return_route = Message.return_route(message)

    case List.last(return_route) do
      nil -> {:error, :return_route_is_empty}
      val -> {:ok, val}
    end
  end

  ## TODO: this is the same as make_responder_init_message in Ockam.SecureChannel.Listener
  defp make_responder_init_message(message, init_handshake) do
    return_route = Message.return_route(message)

    {:ok,
     %Ockam.Message{
       onward_route: [],
       return_route: return_route,
       payload: init_handshake
     }}
  end
end

defmodule Ockam.Identity.SecureChannel.Data do
  @moduledoc """
  Data stage for identity secure channel

  Options:
  - peer_address - address of the channel peer
  - encryption_channel - address of local end of encryption channel
  - identity - own identity
  - contact_id - ID of remote identity
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def inner_setup(options, state) do
    ## TODO: access control to only get secure channel message on the inner address
    peer_address = Keyword.fetch!(options, :peer_address)
    encryption_channel = Keyword.fetch!(options, :encryption_channel)
    identity = Keyword.fetch!(options, :identity)
    contact_id = Keyword.fetch!(options, :contact_id)
    additional_metadata = Keyword.get(options, :additional_metadata, %{})

    inner_address = Map.fetch!(state, :inner_address)

    {:ok,
     Map.merge(
       state,
       %{
         peer_address: peer_address,
         encryption_channel: encryption_channel,
         identity: identity,
         contact_id: contact_id,
         additional_metadata: additional_metadata,
         authorization: %{
           inner_address => [
             :from_secure_channel,
             {:from_addresses, [:message, [encryption_channel]]}
           ]
         }
       }
     )}
  end

  @impl true
  def handle_inner_message(
        message,
        %{address: address, contact_id: contact_id, additional_metadata: additional_metadata} =
          state
      ) do
    with [_me | onward_route] <- Message.onward_route(message),
         [_channel | return_route] <- Message.return_route(message) do
      payload = Message.payload(message)

      ## Assertion. This should be checked by authorization
      %{channel: :secure_channel, source: :channel} = Message.local_metadata(message)

      metadata =
        Map.merge(additional_metadata, %{
          channel: :identity_secure_channel,
          source: :channel,
          ## TODO: rename that to identity_id?
          identity: contact_id
        })

      forwarded_message =
        %Message{
          payload: payload,
          onward_route: onward_route,
          return_route: [address | return_route]
        }
        |> Message.set_local_metadata(metadata)

      Router.route(forwarded_message)
      {:ok, state}
    else
      _other ->
        {:error, {:invalid_inner_message, message}}
    end
  end

  @impl true
  def handle_outer_message(
        message,
        %{encryption_channel: channel, peer_address: peer} = state
      ) do
    case Message.onward_route(message) do
      [_me | onward_route] ->
        forwarded_message =
          message
          |> Message.set_onward_route([channel, peer | onward_route])

        Router.route(forwarded_message)
        {:ok, state}

      _other ->
        {:error, {:invalid_outer_message, message}}
    end
  end
end
