defmodule Ockam.Hub.Service.Discovery.ServiceInfo do
  @moduledoc """
  Service info structure for discovery service.
  """
  defstruct [:id, :route, metadata: %{}]

  @type t() :: %__MODULE__{
          id: binary(),
          route: [Ockam.Address.t()],
          metadata: %{binary() => binary()}
        }
end
