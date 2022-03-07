defmodule Ockam.Hub.Service.Discovery.Storage do
  @moduledoc """
  Storage module behaviour for discovery service
  """
  alias Ockam.Hub.Service.Discovery.ServiceInfo

  @type storage_state() :: any()
  @type metadata() :: %{binary() => binary()}

  @callback init(options :: Keyword.t()) :: storage_state()
  @callback list(storage_state()) :: [ServiceInfo.t()]
  @callback get(id :: binary(), storage_state()) :: {:ok, ServiceInfo.t()} | {:error, :not_found}
  @callback register(id :: binary(), route :: [Ockam.Address.t()], metadata(), storage_state()) ::
              :ok | {:error, reason :: any()}
end
