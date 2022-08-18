defmodule Ockam.Credential.AttributeSet do
  @moduledoc """
  Data struvture representing attribute set:
  group of attributes with common expiration metadata
  """
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:attributes, %{String.t() => binary()}, minicbor: [key: 1])
    field(:expiration, integer(), minicbor: [key: 2])
  end

  def expired?(%__MODULE__{expiration: expiration}) do
    now = System.os_time(:second)
    expiration < now
  end
end
