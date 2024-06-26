defmodule Ockam.Transport.TCP.TransportMessage do
  @moduledoc """
  Ockam messages encoding for TCP transmission
  """
  alias Ockam.Address
  alias Ockam.Message
  alias Ockam.TypedCBOR

  require Logger

  defmodule AddressSchema do
    @moduledoc """
    Ockam Address, cbor encoding
    """
    use TypedStruct

    @address_schema {:struct_values,
                     %{
                       type: %{key: 0, schema: :integer, required: true},
                       value: %{key: 1, schema: :binary, required: true}
                     }}
    def from_cbor_term(term) do
      addr = TypedCBOR.from_cbor_term(@address_schema, term)
      {:ok, Address.denormalize(addr)}
    end

    def to_cbor_term(addr) do
      {:ok, TypedCBOR.to_cbor_term(@address_schema, Address.normalize(addr))}
    end
  end

  defmodule TCPMessage do
    @moduledoc """
    Secure channel message carrying user data
    """
    use TypedStruct

    typedstruct do
      plugin(TypedCBOR.Plugin, encode_as: :list)
      field(:onward_route, list(Address.t()), minicbor: [key: 0, schema: {:list, AddressSchema}])
      field(:return_route, list(Address.t()), minicbor: [key: 1, schema: {:list, AddressSchema}])
      field(:payload, binary(), minicbor: [key: 2])
      field(:tracing_context, String.t() | nil, minicbor: [key: 3, required: false])
    end
  end

  @spec decode(binary()) :: {:ok, Message.t()} | {:error, any()}
  def decode(data) do
    case TCPMessage.decode_strict(data) do
      {:ok, msg} ->
        {:ok,
         %Message{
           onward_route: msg.onward_route,
           return_route: msg.return_route,
           payload: msg.payload,
           local_metadata: %{
             source: :channel,
             channel: :tcp,
             tracing_context: msg.tracing_context
           }
         }}

      error ->
        {:error, {:error_decoding_msg, error}}
    end
  end

  @spec encode(Message.t()) :: {:ok, binary()}
  def encode(%Message{onward_route: o, return_route: r, payload: p, local_metadata: l}) do
    TCPMessage.encode(%TCPMessage{
      onward_route: o,
      return_route: r,
      payload: p,
      tracing_context: Map.get(l, :tracing_context, nil)
    })
  end
end
