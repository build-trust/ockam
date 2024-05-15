defmodule Ockam.SecureChannel.Channel do
  # For now disable this, there are several dependencies that will
  # go away soon.
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies
  @moduledoc """
  Ockam Secure Channel, implementation based on AsymmetricWorker

                +-----------------+                          +-----------------+
                |                 |                          |                 |
            +---+------+    +-----+----+                 +---+------+    +-----+----+
  Plaintext | Address  |    |Inner     |                 |Inner     |    | Address  |  Plaintext
  <------>  |          |    |Address   | <----[...] ---> |Address   |    |          | <------>
            +---+------+    +-----+----+    Ciphertext   +---+------+    +-----+----+
                |  SecureChannel  |                          |   SecureChannel |
                +-----------------+                          +-----------------+


  The secure channel goes through two stages:
    * Handshaking  (noise handshake)
    * Established (channel fully established and peer authenticated)

  At this time, the implementation don't use a proper fsm as that's not directly supported
  by the Worker/AsymmetricWorker machinery.
  """

  use Ockam.AsymmetricWorker
  use TypedStruct

  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage
  alias Ockam.Identity
  alias Ockam.Identity.TrustPolicy
  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Decryptor
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Encryptor
  alias Ockam.SecureChannel.IdentityProof
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol, as: XX
  alias Ockam.SecureChannel.Messages
  alias Ockam.SecureChannel.Messages.PayloadParts
  alias Ockam.Session.Spawner
  alias Ockam.Worker

  alias __MODULE__

  require Logger

  @type encryption_options :: [
          {:static_keypair, %{public: binary(), private: binary()}},
          {:static_key_attestation, binary()}
        ]
  @type authorization :: list() | map()
  @type trust_policies :: list()
  @type secure_channel_opt ::
          {:identity, binary() | :dynamic}
          | {:key_exchange_timeout, non_neg_integer()}
          | {:encryption_options, encryption_options()}
          | {:address, Ockam.Address.t()}
          | {:trust_policies, list(TrustPolicy.trust_rule())}
          | {:authorization, Ockam.Worker.Authorization.config()}
          | {:additional_metadata, map()}
          | {:idle_timeout, non_neg_integer() | :infinity}
          | {:authorities, [Identity.t()]}
          | {:credentials, [binary()]}

  # Note: we could split each of these into their own file as proper modules and delegate
  # the handling of messages to them.  We can do that after the 3-packet handshake that
  # will simplify the handshaking anyway.
  typedstruct module: Handshaking do
    field(:waiting, {pid(), reference()})
    field(:xx, XX.t())
    field(:timer, reference())
  end

  typedstruct module: Established do
    field(:peer_identity, Identity.t())
    field(:peer_identity_id, binary())
    field(:h, binary())
    field(:encrypt_st, Encryptor.t())
    field(:decrypt_st, Decryptor.t())
  end

  # Secure channel' data.  Contains general fields used in every channel state, and
  # the state' specific data (Handshaking/Established)
  typedstruct do
    field(:role, :initiator | :responder)
    field(:identity, Identity.t())
    field(:address, Ockam.Address.t())
    field(:inner_address, Ockam.Address.t())
    field(:peer_route, Ockam.Address.route())
    field(:trust_policies, trust_policies())
    field(:additional_metadata, map())
    field(:channel_state, Handshaking.t() | Established.t())
    field(:payload_parts, map())
    field(:authorities, [Identity.t()])
  end

  @handshake_timeout 30_000

  # 48 * 1024
  @max_payload_size 49_152

  # 60 seconds
  @max_payload_part_update 60

  @type listener_opt ::
          {:responder_authorization, authorization()}
          | secure_channel_opt()

  @type initiator_opt ::
          {:route, Ockam.Address.route()}
          | secure_channel_opt()

  @spec create_listener([listener_opt()]) :: {:ok, Ockam.Address.t()} | {:error, term()}
  def create_listener(opts) do
    Spawner.create(spawner_options(opts))
  end

  @doc """
  Child spec to create listeners

  See create_listener/1
  """
  @spec listener_child_spec([listener_opt()]) :: map()
  def listener_child_spec(args) do
    %{
      id: __MODULE__,
      start: {Spawner, :start_link, [spawner_options(args)]}
    }
  end

  @spec start_link_channel([initiator_opt()], non_neg_integer) ::
          {:ok, pid, any} | {:error, term()}
  def start_link_channel(opts, handshake_timeout \\ @handshake_timeout) do
    ref = make_ref()

    opts =
      Keyword.merge(opts,
        handshake_timeout: handshake_timeout,
        role: :initiator,
        waiter: {self(), ref},
        restart_type: :temporary
      )

    {:ok, pid, addr} = start_link(opts)

    receive do
      {:connected, ^ref} -> {:ok, pid, addr}
    after
      handshake_timeout ->
        Ockam.Node.stop(addr)
        {:error, :key_exchange_timeout}
    end
  end

  @doc "deprecated, use start_link_channel/2"
  @spec create_channel([initiator_opt()], non_neg_integer) ::
          {:ok, Ockam.Address.t()} | {:error, term()}
  def create_channel(opts, handshake_timeout) do
    ref = make_ref()

    opts =
      Keyword.merge(opts,
        handshake_timeout: handshake_timeout,
        role: :initiator,
        waiter: {self(), ref},
        restart_type: :temporary
      )

    {:ok, addr} = create(opts)

    receive do
      {:connected, ^ref} -> {:ok, addr}
    after
      handshake_timeout ->
        Ockam.Node.stop(addr)
        {:error, :key_exchange_timeout}
    end
  end

  @spec get_remote_identity_with_id(Ockam.Address.t()) ::
          {:ok, Ockam.Identity.t(), String.t()} | {:error, any()}
  def get_remote_identity_with_id(worker) do
    Ockam.Worker.call(worker, :get_remote_identity_with_id)
  end

  @spec update_credentials(Ockam.Address.t(), [binary()]) ::
          :ok | {:error, any()}
  def update_credentials(worker, credentials) do
    Ockam.Worker.call(worker, {:update_credentials, credentials})
  end

  @spec get_remote_identity(Ockam.Address.t()) :: {:ok, Ockam.Identity.t()} | {:error, any()}
  def get_remote_identity(worker) do
    Ockam.Worker.call(worker, :get_remote_identity)
  end

  @spec get_remote_identity(Ockam.Address.t()) :: {:ok, String.t()} | {:error, any()}
  def get_remote_identity_id(worker) do
    Ockam.Worker.call(worker, :get_remote_identity_id)
  end

  @doc """
  Stop secure channel and it's remote endpoint
  """
  def disconnect(worker) do
    # TODO: a better solution is needed, this works
    # in a best-effort manner as long as disconnect() is being called
    Ockam.Worker.call(worker, :disconnect)
  end

  def role(worker) do
    Ockam.Worker.call(worker, :role)
  end

  def established?(worker) do
    Ockam.Worker.call(worker, :established?)
  end

  ## AsymmetricWorker callbacks
  @impl true
  def inner_setup(options, %{address: address, inner_address: inner_address} = state) do
    # The authorization rules must apply to the outer address only.  Inner address is
    # ciphertext and can come from tcp transport, from tunneled channel, etc.  The secure
    # channel itself verifies the data comes from the right party.
    # The call below explicitly set the inner_address authorization, that makes any existing
    # authorization already setup on the worker, be applied to the main address instead of all.
    # TODO AsyncWorker could provide out-of-the-box support for this.
    state = Ockam.Worker.update_authorization_state(state, inner_address, [])
    worker_return(inner_setup_impl(address, inner_address, options), state)
  end

  # inner_address is the face pointing the other end (receiving encrypted messages)
  # outer address is the plaintext address
  @impl true
  def handle_inner_message(message, %{state: state} = worker_state) do
    worker_return(handle_inner_message_impl(message, state), worker_state)
  end

  @impl true
  def handle_outer_message(message, %{state: state} = worker_state) do
    worker_return(handle_outer_message_impl(message, state), worker_state)
  end

  ## GenServer
  @impl true
  def handle_call(
        {:update_credentials, credentials},
        _from,
        %{state: %Channel{channel_state: %Established{}}} = state
      ) do
    %{state: c} = state
    %Channel{channel_state: e} = c

    payload = %Messages.RefreshCredentials{
      contact: Identity.get_data(c.identity),
      credentials: credentials
    }

    with {:ok, encrypt_st} <-
           send_payload_over_encrypted_channel(
             payload,
             e.encrypt_st,
             c.peer_route
           ) do
      {:reply, :ok,
       %{state | state: %Channel{c | channel_state: %Established{e | encrypt_st: encrypt_st}}}}
    end
  end

  def handle_call({:update_credentials, _credentials}, _from, state) do
    {:reply, {:error, :handshake_not_finished}, state}
  end

  def handle_call(
        :get_remote_identity,
        _from,
        %{state: %Channel{channel_state: %Established{peer_identity: remote_identity}}} = state
      ) do
    {:reply, {:ok, remote_identity}, state}
  end

  def handle_call(:get_remote_identity, _from, state) do
    {:reply, {:error, :handshake_not_finished}, state}
  end

  @impl true
  def handle_call(
        :get_remote_identity_id,
        _from,
        %{state: %Channel{channel_state: %Established{peer_identity_id: remote_identity_id}}} =
          state
      ) do
    {:reply, {:ok, remote_identity_id}, state}
  end

  def handle_call(:get_remote_identity_id, _from, state) do
    {:reply, {:error, :handshake_not_finished}, state}
  end

  @impl true
  def handle_call(
        :get_remote_identity_with_id,
        _from,
        %{
          state: %Channel{
            channel_state: %Established{
              peer_identity: remote_identity,
              peer_identity_id: remote_identity_id
            }
          }
        } = state
      ) do
    {:reply, {:ok, remote_identity, remote_identity_id}, state}
  end

  def handle_call(:get_remote_identity_with_id, _from, state) do
    {:reply, {:error, :handshake_not_finished}, state}
  end

  def handle_call(
        :disconnect,
        _from,
        %{state: %Channel{channel_state: %Established{} = e} = s} = ws
      ) do
    send_payload_over_encrypted_channel(:close, e.encrypt_st, s.peer_route)
    {:stop, :normal, :ok, ws}
  end

  def handle_call(:established?, _from, %{state: %Channel{channel_state: %Established{}}} = ws) do
    {:reply, true, ws}
  end

  def handle_call(:established?, _from, ws) do
    {:reply, false, ws}
  end

  def handle_call(:role, _from, %{state: %Channel{role: role}} = ws) do
    {:reply, role, ws}
  end

  defp worker_return({:ok, channel_state}, worker_state),
    do: {:ok, Map.put(worker_state, :state, channel_state)}

  defp worker_return({:error, reason}, worker_state), do: {:stop, {:error, reason}, worker_state}

  defp worker_return({:stop, reason, channel_state}, worker_state),
    do: {:stop, reason, Map.put(worker_state, :state, channel_state)}

  defp noise_payloads(:initiator, id_proof), do: %{message3: id_proof}
  defp noise_payloads(:responder, id_proof), do: %{message2: id_proof}

  # TODO:  shouldn't be options if they are mandatory
  defp get_static_keypair(options) do
    Keyword.fetch(options, :static_keypair)
  end

  defp get_static_key_attestation(options) do
    Keyword.fetch(options, :static_key_attestation)
  end

  defp setup_noise_key_exchange(opts, role, identity, credentials) do
    with {:ok, static_keypair} <- get_static_keypair(opts),
         {:ok, attestation} <- get_static_key_attestation(opts) do
      proof = %IdentityProof{
        contact: identity,
        attestation: attestation,
        credentials: credentials
      }

      encoded_proof = IdentityProof.encode(proof)
      payloads = noise_payloads(role, encoded_proof)
      options = [payloads: payloads, static_keypair: static_keypair]
      XX.setup(static_keypair, options)
    end
  end

  defp authorities_form_options(options) do
    {:ok, Keyword.get(options, :authorities, [])}
  end

  defp identity_from_opts(options) do
    Keyword.fetch(options, :identity)
  end

  def inner_setup_impl(address, inner_address, options) do
    trust_policies = Keyword.get(options, :trust_policies, [])
    additional_metadata = Keyword.get(options, :additional_metadata, %{})
    encryption_options = Keyword.get(options, :encryption_options, [])
    key_exchange_timeout = Keyword.get(options, :key_exchange_timeout, @handshake_timeout)

    noise_key_exchange_options =
      Keyword.take(encryption_options, [:static_keypair, :static_key_attestation])

    credentials = Keyword.get(options, :credentials, [])

    with {:ok, role} <- Keyword.fetch(options, :role),
         {:ok, identity} <- identity_from_opts(options),
         {:ok, authorities} <- authorities_form_options(options),
         {:ok, key_exchange_state} <-
           setup_noise_key_exchange(
             noise_key_exchange_options,
             role,
             identity,
             credentials
           ) do
      {:ok, tref} = :timer.apply_after(key_exchange_timeout, Ockam.Node, :stop, [address])

      state = %Channel{
        role: role,
        address: address,
        inner_address: inner_address,
        identity: identity,
        trust_policies: trust_policies,
        additional_metadata: additional_metadata,
        authorities: authorities,
        payload_parts: %{}
      }

      complete_inner_setup(state, options, key_exchange_state, tref)
    end
  end

  defp complete_inner_setup(%Channel{role: :initiator} = state, options, xx, tref) do
    Logger.debug("complete_inner_setup - initiator")

    with {:ok, waiter} <- Keyword.fetch(options, :waiter),
         {:ok, init_route} <- Keyword.fetch(options, :route) do
      continue_handshake({:continue, xx}, %Channel{
        state
        | peer_route: init_route,
          channel_state: %Handshaking{waiting: waiter, timer: tref}
      })
    end
  end

  defp complete_inner_setup(%Channel{role: :responder} = state, options, xx, tref) do
    Logger.debug("complete_inner_setup - responder")

    with {:ok, init_message} <- Keyword.fetch(options, :init_message) do
      handle_inner_message_impl(init_message, %Channel{
        state
        | peer_route: init_message.return_route,
          channel_state: %Handshaking{xx: xx, timer: tref}
      })
    end
  end

  defp next_handshake_state({:continue, xx}, %Channel{channel_state: %Handshaking{} = h} = state) do
    {:ok, %Channel{state | channel_state: %Handshaking{h | xx: xx}}}
  end

  defp next_handshake_state({:complete, {k1, k2, h, rs, payloads}}, state) do
    peer_proof_msg =
      case state.role do
        :initiator -> :message2
        :responder -> :message3
      end

    with {:ok, peer_proof_data} <- Map.fetch(payloads, peer_proof_msg),
         {:ok, identity_proof} <- IdentityProof.decode(peer_proof_data),
         {:ok, peer, peer_identity_id} <- Identity.validate_contact_data(identity_proof.contact),
         {:ok, true} <-
           Identity.verify_purpose_key_attestation(
             peer,
             rs,
             %Ockam.Identity.PurposeKeyAttestation{attestation: identity_proof.attestation}
           ),
         :ok <-
           check_trust(
             state.trust_policies,
             state.identity,
             identity_proof.contact,
             peer_identity_id
           ),
         :ok <-
           process_credentials(
             identity_proof.credentials,
             peer_identity_id,
             state.authorities
           ) do
      {encrypt_st, decrypt_st} = split(k1, k2, state.role)

      {:ok, :cancel} = :timer.cancel(state.channel_state.timer)

      case state.channel_state.waiting do
        {pid, ref} -> send(pid, {:connected, ref})
        nil -> :ok
      end

      established = %Established{
        encrypt_st: encrypt_st,
        decrypt_st: decrypt_st,
        h: h,
        peer_identity: peer,
        peer_identity_id: peer_identity_id
      }

      {:ok, %Channel{state | channel_state: established}}
    else
      error ->
        {:error, {:rejected_identity_proof, error}}
    end
  end

  defp process_credentials([], _peer_identity_id, _authorities), do: :ok

  defp process_credentials([cred], peer_identity_id, authorities) do
    case Identity.verify_credential(peer_identity_id, authorities, cred) do
      {:ok, attribute_set} ->
        AttributeStorage.put_attribute_set(peer_identity_id, attribute_set)

      {:error, reason} ->
        {:error, {:rejected_credential, reason}}
    end
  end

  defp process_credentials(_creds, _peer_identity_id, _authorities),
    do: {:error, :multiple_credentials}

  defp split(k1, k2, :initiator),
    do: {Encryptor.new(k2, 0), Decryptor.new(k1, 0)}

  defp split(k1, k2, :responder),
    do: {Encryptor.new(k1, 0), Decryptor.new(k2, 0)}

  # Check result of the handshake step, send handshake data to the peer if there is a message to exchange,
  # and possible move to another state
  defp continue_handshake({:complete, _key_agreements} = r, state) do
    Logger.debug("continue_handshake - complete")
    next_handshake_state(r, state)
  end

  defp continue_handshake({:continue, key_exchange_state}, state) do
    Logger.debug("continue_handshake - continue")

    with {:ok, data, next} <- XX.out_payload(key_exchange_state) do
      msg = %{
        payload: :bare.encode(data, :data),
        onward_route: state.peer_route,
        return_route: [state.inner_address]
      }

      Worker.route(msg, state)
      next_handshake_state(next, state)
    end
  end

  defp handle_inner_message_impl(message, %Channel{channel_state: %Handshaking{xx: xx}} = state) do
    Logger.debug("handle_inner_message_impl - handshaking")

    with {:ok, data} <- bare_decode_strict(message.payload, :data),
         {:ok, next} <- XX.in_payload(xx, data) do
      continue_handshake(next, %Channel{state | peer_route: message.return_route})
    end
  end

  defp handle_inner_message_impl(message, %Channel{channel_state: channel_state} = state) do
    Logger.debug("handle_inner_message_impl - normal state")

    with {:ok, ciphertext} <- bare_decode_strict(message.payload, :data),
         {:ok, plaintext, decrypt_st} <-
           Decryptor.decrypt("", ciphertext, channel_state.decrypt_st) do
      case Messages.decode(plaintext) do
        {:ok, %Messages.Payload{} = payload} ->
          message = struct(Ockam.Message, Map.from_struct(payload))
          Logger.debug("Received regular message")

          handle_decrypted_message(message, %Channel{
            state
            | channel_state: %{channel_state | decrypt_st: decrypt_st}
          })

        {:ok, %Messages.PayloadPart{} = part} ->
          case handle_inner_message_part(part, state, DateTime.utc_now()) do
            {:ok, message, state} -> handle_decrypted_message(message, state)
            {:ok, state} -> {:ok, state}
          end

        {:ok, :close} ->
          Logger.debug("Peer closed secure channel, terminating #{inspect(state.address)}")
          {:stop, :normal, channel_state}

        ## TODO: add tests
        {:ok, %Messages.RefreshCredentials{contact: contact, credentials: credentials}} ->
          with {:ok, peer_identity, peer_identity_id} <- Identity.validate_contact_data(contact),
               true <- peer_identity_id == channel_state.peer_identity_id,
               :ok <- process_credentials(credentials, peer_identity_id, state.authorities) do
            {:ok,
             %Channel{
               state
               | channel_state: %{
                   channel_state
                   | peer_identity: peer_identity,
                     decrypt_st: decrypt_st
                 }
             }}
          else
            error ->
              Logger.warning("Invalid credential refresh: #{inspect(error)}")
              {:stop, {:error, :invalid_credential_refresh}, state}
          end

        {:error, reason} ->
          {:error, reason}
      end
    else
      # The message couldn't be decrypted.  State remains unchanged
      error ->
        Logger.warning("Failed to decrypt message, discarded: #{inspect(error)}")
        {:ok, state}
    end
  end

  defp handle_inner_message_part(
         %Messages.PayloadPart{
           current_part_number: current_part_number,
           total_number_of_parts: total_number_of_parts,
           payload_uuid: payload_uuid
         } = payload_part,
         %Channel{} = state,
         now
       ) do
    # Get the parts for the current payload UUID
    Logger.debug(
      "Received part #{current_part_number}/#{total_number_of_parts} for message #{payload_uuid}"
    )

    case update_parts(payload_part, state, now) do
      {:error} ->
        state

      {:ok, parts} ->
        case get_complete_payload(parts, state.payload_parts, now, @max_payload_part_update) do
          {:ok, message, state_payload_parts} ->
            Logger.debug(
              "The message #{payload_uuid} is now complete with part #{current_part_number}/#{total_number_of_parts}"
            )

            {:ok, message, %Channel{state | payload_parts: state_payload_parts}}

          {:ok, state_payload_parts} ->
            Logger.debug(
              "The message #{payload_uuid} is not complete with part #{current_part_number}/#{total_number_of_parts}"
            )

            {:ok, %Channel{state | payload_parts: state_payload_parts}}
        end
    end
  end

  # Update the list of received parts with the current part
  # and return the list of updated parts for the current payload UUID
  defp update_parts(
         %Messages.PayloadPart{
           payload_uuid: payload_uuid,
           current_part_number: current_part_number,
           total_number_of_parts: total_number_of_parts
         } = part,
         %Channel{} = state,
         now
       ) do
    case Map.fetch(state.payload_parts, payload_uuid) do
      {:ok, parts} ->
        Logger.debug(
          "Store received part #{current_part_number}/#{total_number_of_parts} for message #{payload_uuid} until all parts have been received"
        )

        case PayloadParts.update(parts, part, now) do
          {:ok, parts} ->
            {:ok, parts}

          {:error, message} ->
            Logger.error(message)
            {:ok, parts}
        end

      :error ->
        if current_part_number > total_number_of_parts do
          Logger.error(
            "The part #{current_part_number}/#{total_number_of_parts} is incorrect: the current_part_number is greater than the total number of parts"
          )

          {:error}
        else
          {:ok, PayloadParts.initialize(part, now)}
        end
    end
  end

  # If the current part completes the payload, return the full message
  # and update the channel state
  def get_complete_payload(
        %Messages.PayloadParts{uuid: uuid} = parts,
        state_payload_parts,
        now,
        max_payload_part_update
      ) do
    case PayloadParts.complete(parts) do
      {:ok, payload} ->
        message = struct(Ockam.Message, Map.from_struct(payload))

        {:ok, message,
         remove_old_parts(Map.delete(state_payload_parts, uuid), now, max_payload_part_update)}

      :error ->
        {:ok,
         remove_old_parts(Map.put(state_payload_parts, uuid, parts), now, max_payload_part_update)}
    end
  end

  defp handle_decrypted_message(message, %Channel{channel_state: %Established{} = e} = state) do
    message
    |> attach_metadata(state.additional_metadata, e)
    |> Message.trace(state.address)
    |> Worker.route(state)

    {:ok, state}
  end

  defp attach_metadata(msg, additional, %Established{peer_identity: i, peer_identity_id: id}) do
    Message.with_local_metadata(
      msg,
      Map.merge(additional, %{identity: i, identity_id: id, channel: :secure_channel})
    )
  end

  defp handle_outer_message_impl(message, %Channel{channel_state: %Established{} = e} = state) do
    message = Message.forward(message)
    Logger.debug("handle_outer_message_impl")

    with {:ok, encrypt_st} <-
           send_over_encrypted_channel(
             message,
             e.encrypt_st,
             state.peer_route
           ) do
      {:ok, %Channel{state | channel_state: %Established{e | encrypt_st: encrypt_st}}}
    end
  end

  defp handle_outer_message_impl(message, state) do
    Logger.warning("discarding message, secure channel not yet established: #{inspect(message)}")
    {:ok, state}
  end

  defp send_over_encrypted_channel(message, encrypt_st, peer_route) do
    Logger.debug("check the size of the payload")
    Logger.debug("check the size of the payload, max size: #{@max_payload_size}")

    if byte_size(message.payload) > @max_payload_size do
      Logger.debug("message size #{byte_size(message.payload)}, max size: #{@max_payload_size}")
      payload_parts = Bits.chunks(message.payload, @max_payload_size)
      payload_uuid = UUID.uuid4()
      total_number_of_parts = length(payload_parts)
      with_index = Enum.with_index(payload_parts)

      List.foldl(with_index, {:ok, encrypt_st}, fn part_index, current_state ->
        {part, index} = part_index
        part_number = index + 1

        Logger.debug(
          "sending part #{part_number}/#{total_number_of_parts} for message #{payload_uuid}"
        )

        payload_part = %Messages.PayloadPart{
          onward_route: message.onward_route,
          return_route: message.return_route,
          payload: part,
          payload_uuid: payload_uuid,
          current_part_number: part_number,
          total_number_of_parts: total_number_of_parts
        }

        send_payload_part_over_encrypted_channel(payload_part, current_state, peer_route)
      end)
    else
      payload = struct(Messages.Payload, Map.from_struct(message))
      send_payload_over_encrypted_channel(payload, encrypt_st, peer_route)
    end
  end

  defp send_payload_part_over_encrypted_channel(payload_part, {:ok, encrypt_st}, peer_route) do
    send_payload_over_encrypted_channel(payload_part, encrypt_st, peer_route)
  end

  defp send_payload_part_over_encrypted_channel(_part, other, _peer_route) do
    other
  end

  defp send_payload_over_encrypted_channel(payload, encrypt_st, peer_route) do
    with {:ok, encoded} <- Messages.encode(payload),
         {:ok, ciphertext, encrypt_st} <- Encryptor.encrypt("", encoded, encrypt_st) do
      ciphertext = :bare.encode(ciphertext, :data)
      envelope = %{onward_route: peer_route, return_route: [], payload: ciphertext}
      Router.route(envelope)
      {:ok, encrypt_st}
    end
  end

  defp check_trust(policies, identity, contact, contact_id) do
    with identity_id <- Identity.get_identifier(identity) do
      TrustPolicy.from_config(policies, %{id: identity_id, identity: identity}, %{
        id: contact_id,
        identity: contact
      })
    end
  end

  defp spawner_options(opts) do
    {opts, worker_opts} = Keyword.split(opts, [:address, :authorization])

    worker_opts =
      worker_opts
      |> Keyword.new(fn
        {:responder_authorization, auth} -> {:authorization, auth}
        other -> other
      end)
      |> Keyword.merge(address_prefix: "ISC_R_", role: :responder, restart_type: :temporary)

    Keyword.merge(opts, worker_mod: __MODULE__, worker_options: worker_opts)
  end

  defp bare_decode_strict(data, type) do
    case :bare.decode(data, type) do
      {:ok, result, ""} -> {:ok, result}
      error -> {:error, {:invalid_bare_data, type, error}}
    end
  end

  # Remove received payload parts when they are too old
  defp remove_old_parts(parts, now, max_payload_part_update) do
    uuids_before = Map.keys(parts)

    result =
      Map.filter(parts, fn {_uuid, ps} ->
        DateTime.add(ps.last_update, max_payload_part_update, :second) >= now
      end)

    uuids_after = Map.keys(parts)

    if length(uuids_after) != length(uuids_after) do
      uuids_removed = uuids_before -- uuids_after
      removed_parts = Map.filter(parts, fn {uuid, _ps} -> uuid in uuids_removed end)

      Logger.warn(
        "Some payload parts are too old and not being tracked anymore: #{inspect(removed_parts)}"
      )
    end

    result
  end
end

defmodule Bits do
  @moduledoc """
  This is a utility module to split a large binary into an enumerable of binaries
  of a given size. The last element might have a size that is small than the requested size
  """

  # Chunk the binary into parts of n _bytes_
  def chunks(binary, n) do
    do_chunks(binary, n * 8, [])
  end

  # Chunk the binary into parts of n _bits_ when there is possibly a leftover
  # of a smaller size
  defp do_chunks(binary, n, acc) when bit_size(binary) <= n do
    Enum.reverse([binary | acc])
  end

  defp do_chunks(binary, n, acc) do
    <<chunk::size(n), rest::bitstring>> = binary
    do_chunks(rest, n, [<<chunk::size(n)>> | acc])
  end
end
