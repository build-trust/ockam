defmodule Ockam.MessageProtocol do
  @moduledoc false
  @type schema() :: any()
  @type extended_schema() :: schema() | [{atom(), schema()}]

  @type protocol_mapping() :: Ockam.Protocol.mapping()

  @type decode_error() :: {:error, {:decode_error, schema :: schema(), result :: any()}}
  @type match_error() ::
          {:error,
           {:unmatched_protocol, type :: String.t()}
           | {:unmatched_subtype, type :: schema(), extended_schema()}}
  @type error() :: decode_error() | match_error()

  @type base_payload() :: %{protocol: String.t(), data: binary()}

  @callback protocol_mapping() :: protocol_mapping()

  def base_schema() do
    {:struct, [protocol: :string, data: :data]}
  end

  @spec decode_payload(binary(), protocol_mapping()) :: {:ok, any()} | error()
  def decode_payload(payload, mapping) do
    in_map = mapping.in

    with {:ok, base_payload} <- base_decode(payload) do
      match_protocol(base_payload, in_map)
    end
  end

  @spec base_decode(binary()) :: {:ok, base_payload()} | error()
  def base_decode(payload) do
    decode(payload, base_schema())
  end

  @spec match_protocol(base_payload(), Ockam.Protocol.schema_map()) ::
          {:ok, type :: String.t(), {atom(), any()} | any()} | error()
  def match_protocol(%{protocol: type, data: data}, map) do
    case Map.fetch(map, type) do
      {:ok, schema} ->
        bare_schema = to_bare_schema(schema)

        with {:ok, decoded} <- decode(data, bare_schema),
             {:ok, matched} <- match_extended_schema(decoded, schema) do
          {:ok, type, matched}
        end

      :error ->
        {:error, {:unmatched_protocol, type}}
    end
  end

  @spec to_bare_schema(extended_schema()) :: schema()
  def to_bare_schema(extended_schema) when is_list(extended_schema) do
    {:union, Keyword.values(extended_schema)}
  end

  def to_bare_schema(extended_schema) do
    extended_schema
  end

  @spec match_extended_schema({atom(), any()} | any(), extended_schema()) ::
          {:ok, {atom(), any()}} | {:ok, any()} | match_error()
  def match_extended_schema({subtype, decoded}, extended_schema) do
    case List.keyfind(extended_schema, subtype, 1) do
      nil -> {:error, {:unmatched_subtype, subtype, extended_schema}}
      {tag, _subtype} -> {:ok, {tag, decoded}}
    end
  end

  def match_extended_schema(decoded, _schema) do
    {:ok, decoded}
  end

  @spec encode_payload(atom(), :request | :response, any()) :: binary()
  def encode_payload(module, direction, data)
      when direction == :request or direction == :response do
    %Ockam.Protocol{name: name} = protocol = module.protocol()

    case Map.get(protocol, direction) do
      nil -> raise("#{direction} not defined in protocol #{inspect(protocol)}")
      schema -> encode_extended(data, name, schema)
    end
  end

  @spec encode_payload(String.t(), any(), protocol_mapping()) :: binary()
  def encode_payload(type, data, mapping) do
    out_map = mapping.out

    case Map.get(out_map, type) do
      nil ->
        raise("Spec for OUT type #{inspect(type)} not found in the mapping #{inspect(mapping)}")

      out_schema ->
        encode_extended(data, type, out_schema)
    end
  end

  ## TODO: this might be moved to BARE lib
  def encode_extended({option, data}, type, schema) do
    bare_schema = to_bare_schema(schema)

    to_encode =
      case Keyword.fetch(schema, option) do
        {:ok, option_spec} ->
          {option_spec, data}

        :error ->
          raise("Option #{inspect(option)} not found in spec #{inspect(schema)}")
      end

    encode(to_encode, type, bare_schema)
  end

  def encode_extended(data, type, schema) do
    encode(data, type, schema)
  end

  @spec decode(binary(), schema()) :: {:ok, any()} | decode_error()
  def decode(data, schema) do
    case :bare.decode(data, schema) do
      {:ok, decoded, ""} -> {:ok, decoded}
      other -> {:error, {:decode_error, schema, other}}
    end
  end

  @spec encode(any(), String.t(), schema()) :: binary()
  def encode(data, type, schema) do
    encoded_data = :bare.encode(data, schema)

    :bare.encode(
      %{
        protocol: type,
        data: encoded_data
      },
      base_schema()
    )
  end

  defmacro __using__(_options) do
    quote do
      @behaviour Ockam.MessageProtocol

      def decode_payload(payload) do
        mapping = protocol_mapping()
        Ockam.MessageProtocol.decode_payload(payload, mapping)
      end

      def encode_payload(type, option, data) do
        encode_payload(type, {option, data})
      end

      def encode_payload(type, data) do
        mapping = protocol_mapping()

        Ockam.MessageProtocol.encode_payload(type, data, mapping)
      end
    end
  end
end
