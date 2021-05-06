defmodule Ockam.Protocol do
  @moduledoc """
  Message payload protocol definition and helper functions
  See Ockam.Protocol.Stream and Ockam.Stream.Workers.Stream for examples
  """

  alias Ockam.Bare.Extended, as: BareExtended
  @enforce_keys [:name]
  defstruct [:name, :request, :response]

  @type extended_schema() :: BareExtended.extended_schema()

  @type t() :: %__MODULE__{
          name: String.t(),
          request: extended_schema() | nil,
          response: extended_schema() | nil
        }

  @callback protocol() :: __MODULE__.t()

  @type direction() :: :request | :response

  @base_schema {:struct, [protocol: :string, data: :data]}

  @spec base_decode(binary()) :: {:ok, %{protocol: binary(), data: binary()}} | {:error, any()}
  def base_decode(payload) do
    BareExtended.decode(payload, @base_schema)
  end

  @spec base_encode(binary(), binary()) :: binary()
  def base_encode(name, data) do
    BareExtended.encode(
      %{
        protocol: name,
        data: data
      },
      @base_schema
    )
  end

  @spec encode_payload(protocol_mod :: module(), direction(), data :: any()) :: binary()
  def encode_payload(protocol_mod, direction, data) do
    protocol = protocol_mod.protocol()

    encoded = encode(protocol, direction, data)

    base_encode(protocol.name, encoded)
  end

  @spec encode(protocol :: module() | t(), direction(), data :: any()) :: binary()
  def encode(protocol_mod, direction, data) when is_atom(protocol_mod) do
    protocol = protocol_mod.protocol()
    encode(protocol, direction, data)
  end

  def encode(protocol, direction, data) do
    schema = Map.get(protocol, direction)

    BareExtended.encode(data, schema)
  end

  @spec decode(protocol_mod :: module(), direction(), data :: binary()) :: any()
  def decode(protocol_mod, direction, data) do
    protocol = protocol_mod.protocol()
    schema = Map.get(protocol, direction)

    BareExtended.decode(data, schema)
  end

  @spec decode_payload(protocol_mod :: module(), direction(), data :: binary()) :: any()
  def decode_payload(protocol_mod, direction, data) do
    case base_decode(data) do
      {:ok, %{protocol: _name, data: protocol_data}} ->
        decode(protocol_mod, direction, protocol_data)

      other ->
        raise("Decode error: #{other}")
    end
  end
end
