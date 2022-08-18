defmodule Ockam.Credential.VerifyRequest do
  @moduledoc """
  Request encoder for credential verifier API
  """
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:credential, binary(), minicbor: [key: 1])
    field(:subject_id, String.t(), minicbor: [key: 2])
    field(:authorities, %{String.t() => binary()}, minicbor: [key: 3])
  end
end
