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
  alias Ockam.Identity.SecureChannel.HandshakeMessage
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
        callback_route: [handshake_address],
        authorization: [{:with_metadata, [:message, %{from_pid: self()}]}]
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

    with {:ok, %HandshakeMessage.Request{contact: contact_data, proof: proof}} <-
           HandshakeMessage.decode(payload),
         {:ok, contact, contact_id} <- Identity.validate_contact_data(identity, contact_data),
         :ok <- Identity.verify_signature(contact, proof, auth_hash),
         :ok <- check_trust(contact, contact_id, state) do
      {:ok, peer_address} = get_peer_address(message)

      authorization_options =
        case Keyword.fetch(handshake_options, :authorization) do
          {:ok, authorization} -> [authorization: authorization]
          :error -> []
        end

      state =
        Map.merge(state, %{
          peer_address: peer_address,
          identity: identity
        })

      additional_metadata = Keyword.get(handshake_options, :additional_metadata, %{})

      data_options =
        [
          peer_address: peer_address,
          encryption_channel: enc_channel,
          identity: identity,
          contact_id: contact_id,
          contact: contact,
          additional_metadata: additional_metadata
        ] ++ authorization_options

      {:ready, identity_handshake(HandshakeMessage.Response, state), data_options, state}
    end
  end

  ## TODO: stop responders if handshake is not done in limited time
  ## if initiator fails to handshake, responder will be left hanging
  @impl true
  def handle_responder(handshake_options, message, handshake_state) do
    ## TODO: maybe we need some separate handle_init_message for responder?
    ## Currently using a flag in handshake_state to distinguish between
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
          callback_route: [handshake_address],
          authorization: [{:with_metadata, [:message, %{from_pid: self()}]}]
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

        {:next, identity_handshake(HandshakeMessage.Request, handshake_state), handshake_state}
      end
    end
  end

  def handle_responder_handshake(handshake_options, message, state) do
    %{identity: identity, auth_hash: auth_hash} = state
    payload = Message.payload(message)

    with {:ok, %HandshakeMessage.Response{contact: contact_data, proof: proof}} <-
           HandshakeMessage.decode(payload),
         {:ok, contact, contact_id} <- Identity.validate_contact_data(identity, contact_data),
         :ok <- Identity.verify_signature(contact, proof, auth_hash),
         :ok <- check_trust(contact, contact_id, state),
         {:ok, peer_address} <- get_peer_address(message) do
      enc_channel = Map.get(state, :encryption_channel)

      additional_metadata = Keyword.get(handshake_options, :additional_metadata, %{})

      authorization_options =
        case Keyword.fetch(handshake_options, :responder_authorization) do
          {:ok, authorization} -> [authorization: authorization]
          :error -> []
        end

      data_options =
        [
          peer_address: peer_address,
          encryption_channel: enc_channel,
          identity: identity,
          contact_id: contact_id,
          contact: contact,
          additional_metadata: additional_metadata
        ] ++ authorization_options

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
      HandshakeMessage.encode(
        struct(type, %{
          contact: contact_data,
          proof: proof
        })
      )

    %Message{
      payload: payload,
      onward_route: [encryption_channel, peer_address],
      return_route: [handshake_address],
      local_metadata: %{from_pid: self()}
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
