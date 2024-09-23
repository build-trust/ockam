defmodule Ockam.Services.TokenLeaseManager.Lease do
  @moduledoc false
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:id, String.t(), minicbor: [key: 1])

    field(:issued_for, Ockam.Identity.Identifier.t(),
      minicbor: [schema: Ockam.Identity.Identifier, key: 2]
    )

    field(:issued, integer(), minicbor: [key: 3])
    field(:expires, integer(), minicbor: [key: 4])
    field(:value, String.t(), minicbor: [key: 5])

    field(:status, :active | :revoked,
      minicbor: [schema: {:enum, [active: 0, revoked: 1]}, key: 6]
    )
  end
end
