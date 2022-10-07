defmodule Ockam.Services.Service do
  @moduledoc """
  Data structure to list ockam services
  """

  defstruct [:id, :pid, :address, :all_addresses, :module, :metadata]

  def from_child({id, pid, _type, _modules}) when is_pid(pid) do
    with {:ok, address} <- get_main_address(pid),
         {:ok, metadata} <- get_worker_metadata(pid) do
      all_addresses = Ockam.Node.list_addresses(pid)
      {:ok, module} = Ockam.Node.get_address_module(address)

      {:ok,
       %__MODULE__{
         id: id,
         pid: pid,
         address: address,
         all_addresses: all_addresses,
         module: module,
         metadata: metadata
       }}
    end
  end

  def from_child(_other) do
    {:error, :not_a_service}
  end

  ## TODO: move that to Ockam.Node?
  def get_main_address(pid) do
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
    ## TODO: implement service metadata
    {:ok, %{}}
  end
end
