defmodule Ockam.Services.Discovery.Storage.Supervisor do
  @moduledoc """
  Supervisor based storage, getting services from supervisor children
  """
  @behaviour Ockam.Services.Discovery.Storage

  alias Ockam.API.Discovery.ServiceInfo

  alias Ockam.Services
  alias Ockam.Services.Service

  def init(options) do
    supervisor = Keyword.fetch!(options, :supervisor)
    %{supervisor: supervisor}
  end

  def get(id, %{supervisor: supervisor} = state) do
    {get_service(id, supervisor), state}
  end

  def list(%{supervisor: supervisor} = state) do
    service_infos = list_services(supervisor)

    {service_infos, state}
  end

  def register(_id, _route, _metadata, state) do
    ## TODO: register remote workers with aliases?
    {:ok, state}
  end

  def get_service(id, supervisor) do
    with {:ok, service} <- Services.get_service(id, supervisor) do
      {:ok, service_info(service)}
    end
  end

  def service_info(%Service{id: id, address: address, metadata: metadata}) do
    %ServiceInfo{id: to_string(id), route: [address], metadata: metadata}
  end

  def list_services(supervisor) do
    Services.list_services(supervisor)
    |> Enum.map(&service_info/1)
  end
end
