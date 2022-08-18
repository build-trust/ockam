defmodule Test.Services.CredentialExchangeTest do
  use ExUnit.Case

  alias Ockam.API.Client, as: ApiClient
  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage
  alias Ockam.Identity
  alias Ockam.Identity.SecureChannel
  alias Ockam.Services.API.CredentialExchange

  setup_all do
    {:ok, listener} = SecureChannel.create_listener(identity: :dynamic)
    {:ok, member_identity, member_id} = Identity.create()
    {:ok, channel} = SecureChannel.create_channel(identity: member_identity, route: [listener])

    [channel: channel, member_identity: member_identity, member_id: member_id]
  end

  test "credential api requires identity_id" do
    {:ok, api} = CredentialExchange.create(authorities: [])

    on_exit(fn ->
      Ockam.Node.stop(api)
    end)

    {:ok, resp} = ApiClient.sync_request(:post, "actions/present", "", [api])

    assert %{status: 400, body: body} = resp

    expected_body = CBOR.encode("secure channel required")
    assert body == expected_body
  end

  test "credential api adds attributes", %{member_id: member_id, channel: channel} do
    {:ok, api} =
      CredentialExchange.create(authorities: [], verifier_module: Ockam.Credential.Verifier.Stub)

    on_exit(fn ->
      Ockam.Node.stop(api)
    end)

    attributes = %{"project" => "123", "role" => "member"}
    expiration = System.os_time(:second) + 100
    credential = Ockam.Credential.Verifier.Stub.make_credential(attributes, expiration)

    {:ok, resp} = ApiClient.sync_request(:post, "actions/present", credential, [channel, api])

    assert %{status: 200} = resp

    member_attributes = AttributeStorage.get_attributes(member_id)
    assert attributes == member_attributes
  end
end
