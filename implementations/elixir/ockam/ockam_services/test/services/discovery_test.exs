defmodule Test.Services.DiscoveryTest do
  use ExUnit.Case

  alias Ockam.API.Client.DiscoveryClient

  alias Ockam.Services.API.Discovery
  alias Ockam.Services.Discovery.Storage

  test "in-memory register and list" do
    {:ok, _pid, _address} =
      Discovery.start_link(
        address: "discovery_memory",
        storage: Storage.Memory
      )

    :ok = Ockam.Node.register_address("me")

    :ok = DiscoveryClient.register(["discovery_memory"], "discovered_service", "me", %{})

    {:ok, services} = DiscoveryClient.list_services([], ["discovery_memory"])

    assert [%{id: "discovered_service", route: ["me"]}] = services
  end

  test "supervisor list" do
    supervisor = Test.Services.DiscoveryTest.Supervisor
    {:ok, sup_pid} = Supervisor.start_link([], name: supervisor, strategy: :one_for_one)

    {:ok, _pid, _address} =
      Discovery.start_link(
        address: "discovery_supervisor",
        storage: Storage.Supervisor,
        storage_options: [supervisor: supervisor]
      )

    Supervisor.start_child(
      supervisor,
      Supervisor.child_spec(
        {Test.Services.DiscoveryTest.Service, [address: "discovered_service"]},
        id: :discovered_service
      )
    )

    {:ok, services} = DiscoveryClient.list_services([], ["discovery_supervisor"])

    assert [%{id: "discovered_service", route: ["discovered_service"]}] = services
    ## on_exit happens on a different process
    ## causing the test process to get a shutdown form the supervisor
    ## unlink to avoid error message
    Process.unlink(sup_pid)
  end
end

defmodule Test.Services.DiscoveryTest.Service do
  use Ockam.Worker

  @impl true
  def handle_message(_message, state) do
    {:ok, state}
  end
end
