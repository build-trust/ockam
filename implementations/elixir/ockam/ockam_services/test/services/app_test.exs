defmodule Test.Services.AppTest do
  use ExUnit.Case

  alias Ockam.Services.Service

  test "app runs configured services" do
    Application.put_env(:ockam_services, :service_providers, [Test.Services.AppTest.Provider])
    Application.put_env(:ockam_services, :services, [:my_service])
    Application.put_env(:ockam_services, :services_config_source, nil)

    {:ok, _apps} = Application.ensure_all_started(:ockam_services)

    sup = Ockam.Services

    assert [%Service{id: :my_service, pid: service_pid}] = Ockam.Services.list_services(sup)

    assert service_pid == Ockam.Node.whereis("my_service")

    :ok = Ockam.Services.stop_service("my_service")

    assert nil == Ockam.Node.whereis("my_service")
    assert [] = Ockam.Services.list_services()
  end
end

defmodule Test.Services.AppTest.Provider do
  @behaviour Ockam.Services.Provider

  @impl true
  def services(), do: [:my_service]

  @impl true
  def child_spec(:my_service, _args) do
    {Test.Services.AppTest.MyService, address: "my_service"}
  end
end

defmodule Test.Services.AppTest.MyService do
  use Ockam.Worker

  @impl true
  def handle_message(_message, state) do
    {:ok, state}
  end
end
