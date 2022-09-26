defmodule Test.Services.CredentialExchangeTest do
  use ExUnit.Case

  alias Ockam.API.Client, as: ApiClient
  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage
  alias Ockam.Identity
  alias Ockam.Identity.SecureChannel
  alias Ockam.Services.API.CredentialExchange

  alias Ockam.Services.Tests.TelemetryListener

  @telemetry_table :credential_test_telemetry_listener

  @telemetry_events [
    [:ockam, :credentials, :presented],
    [:ockam, :credentials, :verified]
  ]

  setup_all do
    {:ok, listener} = SecureChannel.create_listener(identity: :dynamic)
    {:ok, member_identity, member_id} = Identity.create()
    {:ok, channel} = SecureChannel.create_channel(identity: member_identity, route: [listener])

    metrics_listener = TelemetryListener.start(@telemetry_table, @telemetry_events)

    on_exit(fn ->
      send(metrics_listener, :stop)
    end)

    [channel: channel, member_identity: member_identity, member_id: member_id]
  end

  test "credential api requires identity_id" do
    {:ok, api} = CredentialExchange.create(authorities: [])

    TelemetryListener.reset(@telemetry_table)

    on_exit(fn ->
      Ockam.Node.stop(api)
    end)

    {:ok, resp} = ApiClient.sync_request(:post, "actions/present", "", [api])

    assert %{status: 400, body: body} = resp

    expected_body = CBOR.encode("secure channel required")
    assert body == expected_body

    metrics = TelemetryListener.get_metrics(@telemetry_table)

    assert [] = metrics
  end

  test "credential api adds attributes", %{member_id: member_id, channel: channel} do
    {:ok, api} =
      CredentialExchange.create(authorities: [], verifier_module: Ockam.Credential.Verifier.Stub)

    TelemetryListener.reset(@telemetry_table)

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

    metrics = TelemetryListener.get_metrics(@telemetry_table)

    assert [
             {[:ockam, :credentials, :presented], _},
             {[:ockam, :credentials, :verified], _}
           ] = Enum.sort(metrics)
  end
end
