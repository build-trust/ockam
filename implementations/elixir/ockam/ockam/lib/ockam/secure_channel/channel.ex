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


  The secure channel goes throw three stages:
    * Handshaking  (noise handshake)
    * IdentityExchange (identity exchange and verification)
    * Established (channel fully established and peer authenticated)

  At this time, the implementation don't use a proper fsm as that's not directly supported
  by the Worker/AsymmetricWorker machinery.
  """

  use Ockam.AsymmetricWorker
  use TypedStruct

  alias Ockam.Identity
  alias Ockam.Identity.TrustPolicy
  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Decryptor
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Encryptor
  alias Ockam.SecureChannel.HandshakeMessage, as: IdentityHandshake
  alias Ockam.SecureChannel.InitHandshake
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol, as: XX
  alias Ockam.SecureChannel.ServiceMessage
  alias Ockam.Session.Spawner
  alias Ockam.Vault
  alias Ockam.Wire

  alias __MODULE__

  require Logger

  @type encryption_options :: [{:vault, Vault.t()}, {:static_keypair, reference()}]
  @type authorization :: list() | map()
  @type trust_policies :: list()
  @type secure_channel_opt ::
          {:identity, binary() | :dynamic}
          | {:key_exchange_timeout, non_neg_integer()}
          #  vault name where identity' private key is located
          | {:vault_name, String.t() | nil}
          | {:identity_module, module()}
          | {:encryption_options, encryption_options()}
          | {:address, Ockam.Address.t()}
          | {:trust_policies, list(TrustPolicy.trust_rule())}
          | {:authorization, Ockam.Worker.Authorization.config()}
          | {:additional_metadata, map()}
          | {:idle_timeout, non_neg_integer() | :infinity}

  # Note: we could split each of these into their own file as proper modules and delegate
  # the handling of messages to them.  We can do that after the 3-packet handshake that
  # will simplify the handshaking anyway.
  typedstruct module: Handshaking do
    field(:vault, Vault.t())
    field(:waiting, {pid(), reference()})
    field(:xx, XX.t())
    field(:timer, reference())
  end

  typedstruct module: IdentityExchange do
    field(:waiting, {pid(), reference()})
    field(:h, binary())
    field(:encrypt_st, XX.Encryptor.t())
    field(:decrypt_st, XX.Decryptor.t())
    field(:timer, reference())
  end

  typedstruct module: Established do
    field(:peer_identity, Identity.t())
    field(:peer_identity_id, binary())
    field(:h, binary())
    field(:encrypt_st, XX.Encryptor.t())
    field(:decrypt_st, XX.Decryptor.t())
  end

  # Secure channel' data.  Contains general fields used in every channel state, and
  # the state' specific data (Handshaking/IdentityExchange/Established)
  typedstruct do
    field(:role, :initiator | :responder)
    field(:identity, Identity.t())
    field(:address, Ockam.Address.t())
    field(:inner_address, Ockam.Address.t())
    field(:vault_name, binary())
    field(:peer_route, Ockam.Address.route())
    field(:peer_identity_route, Ockam.Address.route())
    field(:trust_policies, trust_policies())
    field(:additional_metadata, map())
    field(:channel_state, Handshaking.t() | IdentityExchange.t() | Established.t())
  end

  @handshake_timeout 30_000

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

  @spec start_link_channel([initiator_opt()], non_neg_integer) :: {:ok, pid} | {:error, term()}
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
      {:connected, ^ref} -> {:ok, pid}
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

  def get_remote_identity(worker) do
    Ockam.Worker.call(worker, :get_remote_identity)
  end

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
        :get_remote_identity,
        _form,
        %{state: %Channel{channel_state: %Established{peer_identity: remote_identity}}} = state
      ) do
    {:reply, remote_identity, state}
  end

  def handle_call(:get_remote_identity, _form, state) do
    {:reply, {:error, :handshake_not_finished}, state}
  end

  @impl true
  def handle_call(
        :get_remote_identity_id,
        _form,
        %{state: %Channel{channel_state: %Established{peer_identity_id: remote_identity_id}}} =
          state
      ) do
    {:reply, remote_identity_id, state}
  end

  def handle_call(:get_remote_identity_id, _form, state) do
    {:reply, {:error, :handshake_not_finished}, state}
  end

  def handle_call(
        :disconnect,
        _from,
        %{state: %Channel{channel_state: %Established{} = e} = s} = ws
      ) do
    payload = ServiceMessage.encode!(%ServiceMessage{command: :disconnect})
    msg = %Message{onward_route: [], return_route: [], payload: payload}
    send_over_encrypted_channel(msg, e.encrypt_st, s.peer_route, s.inner_address)
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

  defp setup_noise_key_exchange(vault, opts) do
    m1_payload = m2_payload = m3_payload = ""
    {:ok, e_key_handle} = Vault.secret_generate(vault, type: :curve25519)
    {:ok, e_public_key} = Vault.secret_publickey_get(vault, e_key_handle)
    e = %{private: e_key_handle, public: e_public_key}

    options =
      [
        vault: vault,
        ephemeral_keypair: e,
        message1_payload: m1_payload,
        message2_payload: m2_payload,
        message3_payload: m3_payload
      ] ++ opts

    XX.setup(options)
  end

  defp vault_from_opts(encryption_options) do
    case Keyword.fetch(encryption_options, :vault) do
      {:ok, vault} -> {:ok, vault}
      :error -> Ockam.Vault.Software.init()
    end
  end

  defp identity_from_opts(options) do
    identity_module =
      Keyword.get_lazy(options, :identity_module, &Identity.default_implementation/0)

    case Keyword.fetch(options, :identity) do
      {:ok, :dynamic} ->
        with {:ok, identity, _id} <- Identity.create(identity_module) do
          {:ok, identity}
        end

      {:ok, other} ->
        Identity.make_identity(identity_module, other)

      :error ->
        {:error, :missing_identity}
    end
  end

  def inner_setup_impl(address, inner_address, options) do
    trust_policies = Keyword.get(options, :trust_policies, [])
    additional_metadata = Keyword.get(options, :additional_metadata, %{})
    encryption_options = Keyword.get(options, :encryption_options, [])
    key_exchange_timeout = Keyword.get(options, :key_exchange_timeout, @handshake_timeout)
    vault_name = Keyword.get(options, :vault_name)
    noise_key_exchange_options = Keyword.take(encryption_options, [:static_keypair])

    with {:ok, role} <- Keyword.fetch(options, :role),
         {:ok, vault} <- vault_from_opts(encryption_options),
         {:ok, identity} <- identity_from_opts(options),
         {:ok, key_exchange_state} <- setup_noise_key_exchange(vault, noise_key_exchange_options) do
      {:ok, tref} = :timer.apply_after(key_exchange_timeout, Ockam.Node, :stop, [address])

      state = %Channel{
        role: role,
        address: address,
        inner_address: inner_address,
        identity: identity,
        vault_name: vault_name,
        trust_policies: trust_policies,
        additional_metadata: additional_metadata
      }

      complete_inner_setup(state, options, key_exchange_state, vault, tref)
    end
  end

  defp complete_inner_setup(%Channel{role: :initiator} = state, options, xx, vault, tref) do
    with {:ok, waiter} <- Keyword.fetch(options, :waiter),
         {:ok, init_route} <- Keyword.fetch(options, :route),
         {:ok, data, next} <- XX.out_payload(xx) do
      ## TODO: optimise double encoding of binaries
      payload = :bare.encode(data, :data)
      ## For now the first message is special, it's _not_ just the noise' handshake, it's wrapped
      ## in a InitHandshake struct.  This is going to be removed soon.
      {:ok, extra_payload} = Wire.encode_address(state.inner_address)
      init_msg = InitHandshake.encode(%{handshake: payload, extra_payload: extra_payload})

      msg = %{
        payload: init_msg,
        onward_route: init_route,
        return_route: [state.inner_address]
      }

      Router.route(msg)

      next_handshake_state(next, %Channel{
        state
        | peer_route: init_route,
          peer_identity_route: :unknown,
          channel_state: %Handshaking{
            vault: vault,
            waiting: waiter,
            timer: tref
          }
      })
    end
  end

  defp complete_inner_setup(%Channel{role: :responder} = state, options, xx, vault, tref) do
    with {:ok, init_message} <- Keyword.fetch(options, :init_message),
         {:ok, init_handshake} <- InitHandshake.decode(init_message.payload),
         {:ok, peer_identity_addr} <- Wire.decode_address(init_handshake.extra_payload),
         {:ok, data, ""} <- :bare.decode(init_handshake.handshake, :data),
         {:ok, next} <- XX.in_payload(xx, data) do
      continue_handshake(next, %Channel{
        state
        | peer_route: init_message.return_route,
          peer_identity_route: [peer_identity_addr],
          channel_state: %Handshaking{timer: tref, vault: vault}
      })
    end
  end

  defp next_handshake_state({:continue, xx}, %Channel{channel_state: %Handshaking{} = h} = state) do
    {:ok, %Channel{state | channel_state: %Handshaking{h | xx: xx}}}
  end

  defp next_handshake_state({:complete, {k1, k2, h, _p}}, state) do
    {encrypt_st, decrypt_st} = split(state.channel_state.vault, k1, k2, state.role)
    identity_exchange(state, h, encrypt_st, decrypt_st)
  end

  defp split(vault, k1, k2, :initiator),
    do: {Encryptor.new(vault, k2, 0), Decryptor.new(vault, k1, 0)}

  defp split(vault, k1, k2, :responder),
    do: {Encryptor.new(vault, k1, 0), Decryptor.new(vault, k2, 0)}

  # Check result of the handshake step, send handshake data to the peer if there is a message to exchange,
  # and possible move to another state
  defp continue_handshake({:complete, _key_agreements} = r, state) do
    next_handshake_state(r, state)
  end

  defp continue_handshake({:continue, key_exchange_state}, state) do
    with {:ok, data, next} <- XX.out_payload(key_exchange_state) do
      msg = %{
        payload: :bare.encode(data, :data),
        onward_route: state.peer_route,
        return_route: [state.inner_address]
      }

      Router.route(msg)
      next_handshake_state(next, state)
    end
  end

  defp identity_exchange(%Channel{role: :initiator} = state, h, encrypt_st, decrypt_st) do
    channel_state = identity_exchange_state(h, encrypt_st, decrypt_st, state.channel_state)
    {:ok, %Channel{state | channel_state: channel_state}}
  end

  defp identity_exchange(%Channel{role: :responder} = state, h, encrypt_st, decrypt_st) do
    {:ok, encrypt_st} = send_identity_proof(IdentityHandshake.Request, state, h, encrypt_st)
    channel_state = identity_exchange_state(h, encrypt_st, decrypt_st, state.channel_state)
    {:ok, %Channel{state | channel_state: channel_state}}
  end

  defp identity_exchange_state(h, encrypt_st, decrypt_st, %Handshaking{waiting: w, timer: t}) do
    %IdentityExchange{h: h, encrypt_st: encrypt_st, decrypt_st: decrypt_st, waiting: w, timer: t}
  end

  defp send_identity_proof(type, state, h, encrypt_st) do
    contact_data = Identity.get_data(state.identity)
    {:ok, proof} = Identity.create_signature(state.identity, h, state.vault_name)
    identity_msg = struct(type, %{contact: contact_data, proof: proof})
    encoded_identity_proof = IdentityHandshake.encode(identity_msg)

    msg = %Message{
      payload: encoded_identity_proof,
      onward_route: state.peer_identity_route,
      return_route: [state.inner_address]
    }

    send_over_encrypted_channel(msg, encrypt_st, state.peer_route, state.inner_address)
  end

  defp handle_inner_message_impl(message, %Channel{channel_state: %Handshaking{xx: xx}} = state) do
    with {:ok, data, ""} <- :bare.decode(message.payload, :data),
         {:ok, next} <- XX.in_payload(xx, data) do
      continue_handshake(next, %Channel{state | peer_route: message.return_route})
    end
  end

  defp handle_inner_message_impl(message, %Channel{channel_state: channel_state} = state) do
    with {:ok, ciphertext, ""} <- :bare.decode(message.payload, :data),
         {:ok, plaintext, decrypt_st} <-
           Decryptor.decrypt("", ciphertext, channel_state.decrypt_st) do
      case Wire.decode(plaintext, :secure_channel) do
        {:ok, message} ->
          handle_decrypted_message(message, %Channel{
            state
            | channel_state: %{channel_state | decrypt_st: decrypt_st}
          })

        {:error, reason} ->
          {:error, reason}
      end
    else
      # The message couldn't be decrypted.  State remains unchanged
      error ->
        Logger.warn("Failed to decrypt message, discarded: #{inspect(error)}")
        {:ok, state}
    end
  end

  defp handle_decrypted_message(msg, %Channel{channel_state: %IdentityExchange{} = ie} = state) do
    with {:ok, %{contact: contact, proof: proof}} <- IdentityHandshake.decode(msg.payload),
         {:ok, identity, identity_id} <- Identity.validate_contact_data(state.identity, contact),
         :ok <- Identity.verify_signature(identity, proof, ie.h),
         :ok <- check_trust(state.trust_policies, state.identity, identity, identity_id) do
      # We received and verified Identity from the other end.  For both initiator and responder, this mean
      # we have authenticated the peer and will move to the Established state.  The only difference is
      # that initiator need to finish the handshake by sending proving his own Identity  too.
      case ie.waiting do
        {pid, ref} ->
          send(pid, {:connected, ref})

        nil ->
          :ok
      end

      {:ok, :cancel} = :timer.cancel(ie.timer)

      established = %Established{
        encrypt_st: ie.encrypt_st,
        decrypt_st: ie.decrypt_st,
        h: ie.h,
        peer_identity: identity,
        peer_identity_id: identity_id
      }

      case state.role do
        :initiator ->
          state = %Channel{state | peer_identity_route: msg.return_route}

          {:ok, encrypt_st} =
            send_identity_proof(
              IdentityHandshake.Response,
              state,
              established.h,
              established.encrypt_st
            )

          {:ok,
           %Channel{state | channel_state: %Established{established | encrypt_st: encrypt_st}}}

        :responder ->
          {:ok, %Channel{state | channel_state: established}}
      end
    end
  end

  defp handle_decrypted_message(
         %{onward_route: [], payload: payload} = msg,
         %Channel{channel_state: %Established{}} = state
       ) do
    case ServiceMessage.decode_strict(payload) do
      {:ok, %ServiceMessage{command: :disconnect}} ->
        {:stop, :normal, state}

      _error ->
        {:error, {:unknown_service_msg, msg}}
    end
  end

  defp handle_decrypted_message(message, %Channel{channel_state: %Established{} = e} = state) do
    message
    |> attach_metadata(state.additional_metadata, e)
    |> Message.trace(state.address)
    |> Message.forward()
    |> Router.route()

    {:ok, state}
  end

  defp attach_metadata(msg, additional, %Established{peer_identity: i, peer_identity_id: id}) do
    Message.with_local_metadata(msg, Map.merge(additional, %{identity: i, identity_id: id}))
  end

  defp handle_outer_message_impl(message, %Channel{channel_state: %Established{} = e} = state) do
    message = Message.forward(message)
    message = Message.set_onward_route(message, state.peer_identity_route ++ message.onward_route)

    with {:ok, encrypt_st} <-
           send_over_encrypted_channel(
             message,
             e.encrypt_st,
             state.peer_route,
             state.inner_address
           ) do
      {:ok, %Channel{state | channel_state: %Established{e | encrypt_st: encrypt_st}}}
    end
  end

  defp handle_outer_message_impl(message, state) do
    Logger.warn("discarding message, secure channel not yet established: #{inspect(message)}")
    {:ok, state}
  end

  defp send_over_encrypted_channel(message, encrypt_st, peer_route, inner_address) do
    with {:ok, encoded} <- Wire.encode(message),
         {:ok, ciphertext, encrypt_st} <- Encryptor.encrypt("", encoded, encrypt_st) do
      ciphertext = :bare.encode(ciphertext, :data)
      envelope = %{onward_route: peer_route, return_route: [inner_address], payload: ciphertext}
      Router.route(envelope)
      {:ok, encrypt_st}
    end
  end

  defp check_trust(policies, identity, contact, contact_id) do
    with {:ok, identity_id} <- Identity.validate_identity_change_history(identity) do
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
end
