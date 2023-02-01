defmodule Ockam.Services.API.Tests.FakeCloudService do
  @moduledoc false
  @behaviour Ockam.Services.TokenLeaseManager.CloudService

  alias Ockam.Services.TokenLeaseManager.Lease

  @impl true
  def init(_config), do: {:ok, []}

  @impl true
  def create(_config, identity_id, ttl) do
    now = DateTime.utc_now()
    expires = DateTime.add(now, ttl, :second)

    {:ok,
     %Lease{
       id: "ID_#{:rand.uniform(65536)}",
       issued: DateTime.to_iso8601(now),
       issued_for: identity_id,
       expires: DateTime.to_iso8601(expires),
       value: "TOKEN_#{:rand.uniform(65536)}",
       status: "active"
     }}
  end

  @impl true
  def revoke(_config, _token_id), do: :ok
  @impl true
  def get_all(_config), do: {:ok, []}
end

defmodule Ockam.Services.TokenLeaseManager.Test do
  use ExUnit.Case

  alias Ockam.API.Client
  alias Ockam.Identity
  alias Ockam.Identity.SecureChannel
  alias Ockam.Services.TokenLeaseManager
  alias Ockam.Services.TokenLeaseManager.Lease
  alias Ockam.Services.TokenLeaseManager.StorageService.Memory, as: MemoryStorage
  alias Ockam.Vault.Software, as: SoftwareVault

  setup do
    {:ok, lm} =
      TokenLeaseManager.create(
        address: "lease_manager",
        cloud_service: {Ockam.Services.API.Tests.FakeCloudService, []},
        storage_service_module: MemoryStorage,
        ttl: 60 * 60
      )

    {:ok, short_live_lm} =
      TokenLeaseManager.create(
        address: "short_lived_lease_manager",
        cloud_service: {Ockam.Services.API.Tests.FakeCloudService, []},
        storage_service_module: MemoryStorage,
        ttl: 1
      )

    on_exit(fn ->
      Ockam.Node.stop(lm)
      Ockam.Node.stop(short_live_lm)
    end)

    [lm: lm, short_live_lm: short_live_lm]
  end

  test "create and list leases", %{lm: lm} do
    {:ok, vault} = SoftwareVault.init()
    {:ok, listener_identity, _id} = Identity.create(Ockam.Identity.Stub)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [vault: vault]
      )

    {:ok, bob, bob_id} = Identity.create(Ockam.Identity.Stub)
    {:ok, alice, alice_id} = Identity.create(Ockam.Identity.Stub)

    {:ok, bob_channel} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [vault: vault],
        route: [listener]
      )

    {:ok, alice_channel} =
      SecureChannel.create_channel(
        identity: alice,
        encryption_options: [vault: vault],
        route: [listener]
      )

    # Initially no leases
    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, [], ""} = Lease.decode_list(body)

    {:ok, resp} = Client.sync_request(:post, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, %Lease{issued_for: ^bob_id} = bob_lease1} = Lease.decode_strict(body)

    {:ok, resp} = Client.sync_request(:post, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, %Lease{} = bob_lease2} = Lease.decode_strict(body)

    # Bob has two leases
    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    {:ok, bob_leases, ""} = Lease.decode_list(body)
    assert Enum.sort([bob_lease1, bob_lease2]) == Enum.sort(bob_leases)

    # Alice can't see bob' leases
    {:ok, resp} = Client.sync_request(:get, "/", nil, [alice_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, [], ""} = Lease.decode_list(body)

    # Alice can request his own lease
    {:ok, resp} = Client.sync_request(:post, "/", nil, [alice_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, %Lease{issued_for: ^alice_id} = alice_lease} = Lease.decode_strict(body)

    {:ok, resp} = Client.sync_request(:get, "/", nil, [alice_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, [^alice_lease], ""} = Lease.decode_list(body)
  end

  test "lease get", %{lm: lm} do
    {:ok, vault} = SoftwareVault.init()
    {:ok, listener_identity, _id} = Identity.create(Ockam.Identity.Stub)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [vault: vault]
      )

    {:ok, bob, bob_id} = Identity.create(Ockam.Identity.Stub)
    {:ok, alice, _alice_id} = Identity.create(Ockam.Identity.Stub)

    {:ok, bob_channel} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [vault: vault],
        route: [listener]
      )

    {:ok, alice_channel} =
      SecureChannel.create_channel(
        identity: alice,
        encryption_options: [vault: vault],
        route: [listener]
      )

    {:ok, resp} = Client.sync_request(:post, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp

    assert {:ok, %Lease{id: bob_lease_1_id, issued_for: ^bob_id} = bob_lease1} =
             Lease.decode_strict(body)

    {:ok, resp} = Client.sync_request(:get, "/#{bob_lease_1_id}", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, ^bob_lease1} = Lease.decode_strict(body)

    # Alice can't retrieve bob' lease
    assert {:ok, %{status: 404, body: _}} =
             Client.sync_request(:get, "/#{bob_lease_1_id}", nil, [alice_channel, lm])
  end

  test "lease revoke", %{lm: lm} do
    {:ok, vault} = SoftwareVault.init()
    {:ok, listener_identity, _id} = Identity.create(Ockam.Identity.Stub)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [vault: vault]
      )

    {:ok, bob, bob_id} = Identity.create(Ockam.Identity.Stub)
    {:ok, alice, _alice_id} = Identity.create(Ockam.Identity.Stub)

    {:ok, bob_channel} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [vault: vault],
        route: [listener]
      )

    {:ok, resp} = Client.sync_request(:post, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, %Lease{id: bob_lease_1_id, issued_for: ^bob_id}} = Lease.decode_strict(body)
    {:ok, resp} = Client.sync_request(:post, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, %Lease{issued_for: ^bob_id}} = Lease.decode_strict(body)

    {:ok, resp} = Client.sync_request(:delete, "/#{bob_lease_1_id}", nil, [bob_channel, lm])
    assert %{status: 200, body: nil} = resp

    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, [%Lease{id: bob_lease_2_id}], ""} = Lease.decode_list(body)

    # Alice can't delete bob' lease
    {:ok, alice_channel} =
      SecureChannel.create_channel(
        identity: alice,
        encryption_options: [vault: vault],
        route: [listener]
      )

    {:ok, resp} = Client.sync_request(:delete, "/#{bob_lease_2_id}", nil, [alice_channel, lm])
    assert %{status: 404} = resp

    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, [%Lease{id: ^bob_lease_2_id}], ""} = Lease.decode_list(body)
  end

  test "lease expiration", %{short_live_lm: short_live_lm} do
    {:ok, vault} = SoftwareVault.init()
    {:ok, listener_identity, _id} = Identity.create(Ockam.Identity.Stub)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [vault: vault]
      )

    {:ok, bob, bob_id} = Identity.create(Ockam.Identity.Stub)

    {:ok, bob_channel} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [vault: vault],
        route: [listener]
      )

    {:ok, resp} = Client.sync_request(:post, "/", nil, [bob_channel, short_live_lm])
    assert %{status: 200, body: body} = resp
    assert {:ok, %Lease{issued_for: ^bob_id} = bob_lease1} = Lease.decode_strict(body)

    # Bob has the lease
    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, short_live_lm])
    assert %{status: 200, body: body} = resp
    {:ok, [^bob_lease1], ""} = Lease.decode_list(body)
    Process.sleep(2000)

    # Lease expire (its removed from backed service as well)
    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, short_live_lm])
    assert %{status: 200, body: body} = resp
    {:ok, [], ""} = Lease.decode_list(body)
  end
end
