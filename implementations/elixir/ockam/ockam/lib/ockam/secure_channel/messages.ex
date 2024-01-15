defmodule Ockam.SecureChannel.Messages do
  @moduledoc """
  Secure Channel protocol Messages
  """
  alias Ockam.Address
  alias Ockam.SecureChannel.Messages.RefreshCredentials
  alias Ockam.TypedCBOR

  require Logger

  defmodule AddressSchema do
    @moduledoc """
    Ockam Address, cbor encoding
    """
    use TypedStruct

    @address_schema {:struct,
                     %{
                       type: %{key: 1, schema: :integer, required: true},
                       value: %{key: 2, schema: :charlist, required: true}
                     }}
    def from_cbor_term(term) do
      addr = TypedCBOR.from_cbor_term(@address_schema, term)
      {:ok, Address.denormalize(addr)}
    end

    def to_cbor_term(addr) do
      {:ok, TypedCBOR.to_cbor_term(@address_schema, Address.normalize(addr))}
    end
  end

  defmodule Payload do
    @moduledoc """
    Secure channel message carrying user data
    """
    use TypedStruct

    typedstruct do
      plugin(TypedCBOR.Plugin, encode_as: :list)
      field(:onward_route, list(Address.t()), minicbor: [key: 0, schema: {:list, AddressSchema}])
      field(:return_route, list(Address.t()), minicbor: [key: 1, schema: {:list, AddressSchema}])
      field(:payload, binary(), minicbor: [key: 2])
    end
  end

  defmodule RefreshCredentials do
    @moduledoc """
    Secure channel message refreshing sender credentials
    """
    defstruct [:contact, :credentials]

    def from_cbor_term([change_history, credentials]) do
      {:ok,
       %RefreshCredentials{
         contact: CBOR.encode(change_history),
         credentials: Enum.map(credentials, fn c -> CBOR.encode(c) end)
       }}
    end

    def to_cbor_term(%RefreshCredentials{contact: contact, credentials: credentials}) do
      {:ok, contact, ""} = CBOR.decode(contact)

      credentials =
        Enum.map(credentials, fn c ->
          {:ok, d, ""} = CBOR.decode(c)
          d
        end)

      {:ok, [contact, credentials]}
    end
  end

  @enum_schema {:variant_enum,
                [
                  {Ockam.SecureChannel.Messages.Payload, 0},
                  {Ockam.SecureChannel.Messages.RefreshCredentials, 1},
                  close: 2
                ]}

  def decode(encoded) do
    TypedCBOR.decode_strict(@enum_schema, encoded)
  end

  def encode(msg) do
    TypedCBOR.encode(@enum_schema, msg)
  end
end
