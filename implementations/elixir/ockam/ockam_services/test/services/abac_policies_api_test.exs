defmodule Ockam.Services.API.ABAC.PoliciesApi.Test do
  @moduledoc false
  use ExUnit.Case

  alias Ockam.ABAC.ActionId
  alias Ockam.ABAC.PolicyCheck
  alias Ockam.ABAC.PolicyStorage
  alias Ockam.ABAC.Request

  alias Ockam.API.Client, as: ApiClient
  alias Ockam.Services.API.ABAC.PoliciesApi

  setup do
    Application.put_env(:ockam_abac, :policy_storage, Ockam.ABAC.PolicyStorage.ETS)
    {:ok, _storage} = Ockam.ABAC.PolicyStorage.ETS.start_link([])
    {:ok, api} = PoliciesApi.create()

    on_exit(fn ->
      Application.delete_env(:ockam_abac, :policy_storage)
      Ockam.Node.stop(api)
    end)

    {:ok, [api: api]}
  end

  test "set policy", %{api: api} do
    action_id = ActionId.new("test", "set_policy")
    assert {:error, :not_found} = PolicyStorage.ETS.get_policy(action_id)

    {:ok, %{status: 200}} =
      ApiClient.sync_request(:put, ActionId.format(action_id), "(= subject.foo 1)", [api])

    assert {:ok, policy} = PolicyStorage.ETS.get_policy(action_id)

    request = %Request{
      action_id: action_id,
      subject_attributes: %{"foo" => 1},
      resource_attributes: %{},
      action_attributes: %{}
    }

    assert :ok == PolicyCheck.match_policies(request, [policy])
  end

  test "get policy", %{api: api} do
    action_id = ActionId.new("test", "get_policy")

    {:ok, %{status: 200}} =
      ApiClient.sync_request(:put, ActionId.format(action_id), "(= subject.foo 1)", [api])

    {:ok, %{status: 200, body: "(= subject.foo 1)"}} =
      ApiClient.sync_request(:get, ActionId.format(action_id), "", [api])
  end

  test "delete policy", %{api: api} do
    action_id = ActionId.new("test", "get_policy")

    {:ok, %{status: 200}} =
      ApiClient.sync_request(:put, ActionId.format(action_id), "(= subject.foo 1)", [api])

    {:ok, %{status: 200}} = ApiClient.sync_request(:delete, ActionId.format(action_id), "", [api])

    assert {:error, :not_found} = PolicyStorage.ETS.get_policy(action_id)
  end
end
