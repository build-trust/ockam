defmodule Ockam.Services.Relay.Types do
  defmodule CreateRelayRequest do
    @moduledoc false
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:alias, String.t(), minicbor: [key: 1])
      field(:tags, %{String.t() => String.t()}, minicbor: [key: 2])
    end
  end

  defmodule Relay do
    @moduledoc false

    use TypedStruct

    alias Ockam.Services.Relay.Types.CBORUnixTimestamp

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:addr, String.t(), minicbor: [key: 1])
      field(:tags, %{String.t() => String.t()}, minicbor: [key: 2])
      field(:target_identifier, binary(), minicbor: [key: 3, schema: Ockam.Identity.Identifier])
      field(:created_at, integer(), minicbor: [key: 4, schema: CBORUnixTimestamp])
      field(:updated_at, integer(), minicbor: [key: 5, schema: CBORUnixTimestamp])
    end

    def from_registry_attributes({addr, attrs}) do
      struct(Relay, Map.put(attrs, :addr, addr))
    end
  end

  defmodule CBORUnixTimestamp do
    @moduledoc false
    def from_cbor_term(val), do: DateTime.from_unix(val)

    def to_cbor_term(datetime), do: {:ok, DateTime.to_unix(datetime, :second)}
  end
end
