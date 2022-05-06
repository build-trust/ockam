defmodule Ockam.Services.Discovery.Storage.Supervisor do
  @moduledoc """
  Supervisor based storage, getting services from supervisor children
  """
  @behaviour Ockam.Services.Discovery.Storage

  alias Ockam.API.Discovery.ServiceInfo

  def init(options) do
    supervisor = Keyword.fetch!(options, :supervisor)
    %{supervisor: supervisor}
  end

  def get(id, %{supervisor: supervisor} = state) do
    children = Supervisor.which_children(supervisor)

    case List.keyfind(children, String.to_atom(id), 0) do
      {_id, pid, _type, _modules} ->
        {proc_service_info(id, pid), state}

      nil ->
        {{:error, :not_found}, state}
    end
  end

  def proc_service_info(id, pid) do
    with {:ok, address} <- get_worker_address(pid),
         {:ok, metadata} <- get_worker_metadata(pid) do
      {:ok, %ServiceInfo{id: to_string(id), route: [address], metadata: metadata}}
    end
  end

  def get_worker_address(pid) do
    case Ockam.Node.list_addresses(pid) do
      [address] ->
        {:ok, address}

      [_ | _] ->
        try do
          {:ok, Ockam.Worker.get_address(pid)}
        catch
          _type, {reason, _call} ->
            {:error, reason}

          type, reason ->
            {:error, {type, reason}}
        end

      [] ->
        {:error, :not_found}
    end
  end

  def get_worker_metadata(_pid) do
    ## TODO: implement metadata in workers
    {:ok, %{}}
  end

  def list(%{supervisor: supervisor} = state) do
    service_infos =
      supervisor
      |> Supervisor.which_children()
      |> Enum.map(fn {id, pid, _type, _modules} ->
        proc_service_info(id, pid)
      end)
      |> Enum.flat_map(fn
        {:ok, service_info} -> [service_info]
        {:error, _reason} -> []
      end)

    {service_infos, state}
  end

  def register(_id, _route, _metadata, state) do
    ## TODO: register remote workers with aliases?
    {:ok, state}
  end
end
