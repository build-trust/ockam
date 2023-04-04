defmodule Test.Services.API.NodeInfoTest do
  use ExUnit.Case

  alias Ockam.API.Client, as: ApiClient
  alias Ockam.Services.API.NodeInfo

  test "node info with version" do
    {:ok, address} = NodeInfo.create(info: %NodeInfo.Info{version: "foo"})

    {:ok, response} = ApiClient.sync_request(:get, "", "", [address])

    assert {:ok, %NodeInfo.Info{version: "foo"}, ""} = NodeInfo.Info.decode(response.body)
  end

  test "node info without version crashes" do
    assert {:error, _reason} = NodeInfo.create()
    assert {:error, _reason} = NodeInfo.create(info: %{})
    assert {:error, _reason} = NodeInfo.create(info: %NodeInfo.Info{})

    Process.flag(:trap_exit, true)
    {:ok, pid, _info} = NodeInfo.start_link([])
    assert_receive {:EXIT, pid, _reason}, 5_000
  end
end
