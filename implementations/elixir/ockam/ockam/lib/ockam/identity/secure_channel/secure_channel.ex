defmodule Ockam.Identity.SecureChannel do
  @moduledoc """
  Functions to start identity secure channel and listener
  """

  alias Ockam.Identity

  alias Ockam.Session.Pluggable.Initiator
  alias Ockam.Session.Pluggable.Responder

  alias Ockam.Session.Spawner

  require Logger

  @doc """
  Start an identity secure channel listener.

  Options:
  - identity :: binary() | :dynamic - identity to use in spawned channels, :dynamic will generate a new identity
  - identity_module (optional) :: module() - module to generate dynamic identity
  - encryption_options (optional) :: Keyword.t() - options for Ockam.SecureChannel.Channel
  - address (optional) :: Ockam.Address.t() - listener address
  - trust_policies (optional) :: list() - trust policy configuration
  - authorization (optional) :: list() | map() - listener authorization configuration
  - responder_authorization (optional) :: list() | map() - responders authorization configuration
  - additional_metadata (optional) :: map() - metadata to add to outgoing messages

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

  See create_listener/1
  """
  def listener_child_spec(args) do
    spawner_options = spawner_options(args)

    %{
      id: __MODULE__,
      start: {Spawner, :start_link, [spawner_options]}
    }
  end

  defp spawner_options(options) do
    listener_keys = [:address, :inner_address, :restart_type, :authorization]
    handshake_options = Keyword.drop(options, listener_keys)
    idle_timeout = Keyword.get(options, :idle_timeout, :infinity)

    responder_options = [
      address_prefix: "ISC_R_",
      worker_mod: Ockam.Identity.SecureChannel.Data,
      handshake: Ockam.Identity.SecureChannel.Handshake,
      handshake_options: handshake_options,
      idle_timeout: idle_timeout,
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

    with {:ok, identity} <- get_identity(handshake_options) do
      new_handshake_options = Keyword.put(handshake_options, :identity, identity)
      new_worker_options = Keyword.put(worker_options, :handshake_options, new_handshake_options)
      {:ok, Keyword.put(options, :worker_options, new_worker_options), state}
    end
  end

  defp get_identity(options) do
    identity_module = Keyword.get(options, :identity_module, Identity.default_implementation())

    case Keyword.fetch(options, :identity) do
      {:ok, :dynamic} ->
        {:ok, identity, _id} = Identity.create(identity_module)
        {:ok, identity}

      {:ok, other} ->
        {:ok, identity} = Identity.make_identity(identity_module, other)
        {:ok, identity}

      :error ->
        {:error, :identity_option_missing}
    end
  end

  @doc """
  Start an identity secure channel.

  Options:
  - identity :: binary() | :dynamic - identity to use in the channel, :dynamic will generate a new identity
  - route :: Ockam.Address.route() - route to connect to
  - identity_module (optional) :: module() - module to generate dynamic identity defaults to `Ockam.Identity.default_implementation()`
  - encryption_options (optional) :: Keyword.t() - options for Ockam.SecureChannel.Channel
  - address (optional) :: Ockam.Address.t() - initiator address
  - trust_policies (optional) :: list() - trust policy configuration
  - authorization (optional) :: list() | map() - initiator authorization configuration
  - additional_metadata (optional) :: map() - metadata to add to outgoing messages

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

    with {:ok, identity} <- get_identity(options) do
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

  @doc """
  Retrieve remote endpoint identity.

  This identity is added to all messages coming from the channel
  """
  def get_remote_identity(worker) do
    Ockam.Worker.call(worker, :get_remote_identity)
  end

  @doc """
  Retrieve remote endpoint identity identifier.

  This identifier is added to all messages coming from the channel
  """
  def get_remote_identity_id(worker) do
    Ockam.Worker.call(worker, :get_remote_identity_id)
  end
end
