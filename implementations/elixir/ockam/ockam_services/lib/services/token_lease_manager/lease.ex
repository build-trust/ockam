defmodule Ockam.Services.TokenLeaseManager.Lease do
  @moduledoc false
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:id, String.t(), minicbor: [key: 1])
    field(:issued_for, String.t(), minicbor: [key: 2])
    field(:issued, String.t(), minicbor: [key: 3])
    field(:expires, String.t(), minicbor: [key: 4])
    field(:value, String.t(), minicbor: [key: 5])
    field(:status, String.t(), minicbor: [key: 6])
  end
end
