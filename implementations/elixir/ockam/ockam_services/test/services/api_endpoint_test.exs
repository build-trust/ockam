defmodule Ockam.Services.API.Tests.EndpointAPI do
  @moduledoc false
  use Ockam.Services.API.Endpoint

  alias Ockam.API.Request
  @impl true
  def routes() do
    [
      {:all, :get, "/", &list/3},
      {:member, :get, "/:id/:role", &show/3},
      {:admin, :put, "/item/:id/:role", &edit/3}
    ]
  end

  def list(_req, %{}, %{}), do: {:ok, "LIST"}
  def show(_req, %{id: id}, %{auth_data: auth_data}), do: {:ok, id <> auth_data}
  def edit(%Request{body: body}, %{id: _id}, %{}), do: {:ok, body}

  # Note: an actual implementation will look at the identity information attached to the request,
  # for example, to perform authentication.  Here we just pass a "role" in the url as it's easier to setup
  # the test.
  @impl true
  def authorize(:all, _req, _bindings), do: true
  def authorize(:member, _req, %{id: "a", role: "member"}), do: {true, %{auth_data: "EXTRA"}}
  def authorize(:admin, _req, %{id: "a", role: "admin"}), do: true
  def authorize(_auth_type, _req, _bindings), do: false
end

defmodule Ockam.Services.API.Tests.Endpoint do
  use ExUnit.Case

  alias Ockam.API.Client
  alias Ockam.Services.API.Tests.EndpointAPI

  setup_all do
    {:ok, api} = EndpointAPI.create(address: "endpoint")
    [api: api]
  end

  test "list all authorized", %{api: api} do
    {:ok, resp} = Client.sync_request(:get, "/", nil, [api])
    assert %{status: 200, body: "LIST"} = resp
  end

  test "get authorized", %{api: api} do
    {:ok, resp} = Client.sync_request(:get, "/a/member", nil, [api])
    assert %{status: 200, body: "aEXTRA"} = resp
  end

  test "get not authorized", %{api: api} do
    {:ok, resp} = Client.sync_request(:get, "/a/other", nil, [api])
    assert %{status: 401} = resp
    {:ok, resp} = Client.sync_request(:get, "/b/member", nil, [api])
    assert %{status: 401} = resp
  end

  test "put authorized", %{api: api} do
    {:ok, resp} = Client.sync_request(:put, "/item/a/admin", "SOME", [api])
    assert %{status: 200, body: "SOME"} = resp
  end

  test "put not authorized", %{api: api} do
    {:ok, resp} = Client.sync_request(:put, "/item/a/member", "SOME", [api])
    assert %{status: 401} = resp
  end

  test "not found", %{api: api} do
    {:ok, resp} = Client.sync_request(:get, "/item", nil, [api])
    assert %{status: 404} = resp
  end
end
