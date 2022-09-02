defmodule Ockam.Credential.AttributeSet do
  @moduledoc """
  Data struvture representing attribute set:
  group of attributes with common expiration metadata
  """

  use TypedStruct

  defmodule Attributes do
    @moduledoc """
    Attributes are returned by verifier as an embedded struct,
    with a single field (a string() -> binary() map)
    """
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:attributes, %{String.t() => binary()}, minicbor: [key: 1])
    end
  end

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:attributes, Attributes.t(), minicbor: [key: 1, schema: Attributes.minicbor_schema()])
    field(:expiration, integer(), minicbor: [key: 2])
  end

  def expired?(%__MODULE__{expiration: expiration}) do
    now = System.os_time(:second)
    expiration < now
  end
end
