defmodule Ockam.Services.TokenLeaseManager.Storage.Memory.Test do
  use ExUnit.Case

  alias Ockam.Identity
  alias Ockam.Services.TokenLeaseManager.Lease
  alias Ockam.Services.TokenLeaseManager.StorageService.Memory, as: Storage

  setup do
    {:ok, identity1} = Identity.create()
    id1 = Identity.get_identifier(identity1)
    {:ok, identity2} = Identity.create()
    id2 = Identity.get_identifier(identity2)
    {:ok, identity3} = Identity.create()
    id3 = Identity.get_identifier(identity3)

    leases = [
      %Lease{id: "1", value: "vvvv", issued_for: id1},
      %Lease{id: "2", value: "vvvv", issued_for: id2},
      %Lease{id: "3", value: "vvvv", issued_for: id1}
    ]

    {:ok, state} = Ockam.Services.TokenLeaseManager.StorageService.Memory.init(leases: leases)

    [state: state, id1: id1, id2: id2, id3: id3]
  end

  test "get lease", %{state: state, id1: id1} do
    assert {:ok, %Lease{id: "3"}} = Storage.get(state, id1, "3")
    assert {:ok, nil} = Storage.get(state, id1, "4")
  end

  test "list leases", %{state: state, id1: id1, id2: id2, id3: id3} do
    assert {:ok, []} = Storage.get_all(state, id3)
    assert {:ok, [%Lease{id: "2"}]} = Storage.get_all(state, id2)
    {:ok, r} = Storage.get_all(state, id1)
    assert [%Lease{id: "1"}, %Lease{id: "3"}] = Enum.sort(r)
  end

  test "store leases", %{state: state, id2: id2, id3: id3} do
    :ok = Storage.save(state, %Lease{id: "4", issued_for: id2})
    :ok = Storage.save(state, %Lease{id: "5", issued_for: id3})
    assert {:ok, [%Lease{id: "5"}]} = Storage.get_all(state, id3)
    {:ok, r} = Storage.get_all(state, id2)
    assert [%Lease{id: "2"}, %Lease{id: "4"}] = Enum.sort(r)
  end

  test "remove leases", %{state: state, id1: id1, id2: id2} do
    :ok = Storage.remove(state, id1, "1")
    :ok = Storage.remove(state, id2, "2")
    assert {:ok, []} = Storage.get_all(state, id2)
    assert {:ok, [%Lease{id: "3"}]} = Storage.get_all(state, id1)
  end
end
