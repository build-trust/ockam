defmodule Ockam.Bare.Union do
  @moduledoc """
  Extension for BARE schema

  Support simple tags for union types,
  Union type can be defined as [type1: schema, type2: schema]
  and can be encoded and decoded from/to {:type1, data} or {:type2, data}

  """

  @type schema() :: :bare.spec()
  @type extended_schema() :: schema() | [{atom(), schema()}]

  @type match_error() :: {:error, {:unmatched_subtype, atom(), extended_schema()}}

  ## TODO: this might be moved to BARE lib
  def encode({option, data}, schema) do
    bare_schema = to_bare_schema(schema)

    to_encode =
      case Keyword.fetch(schema, option) do
        {:ok, option_spec} ->
          {option_spec, data}

        :error ->
          raise("Option #{inspect(option)} not found in spec #{inspect(schema)}")
      end

    :bare.encode(to_encode, bare_schema)
  end

  def encode(data, schema) do
    bare_schema = to_bare_schema(schema)

    :bare.encode(data, bare_schema)
  end

  def decode(data, extended_schema) do
    bare_schema = to_bare_schema(extended_schema)

    case :bare.decode(data, bare_schema) do
      {:ok, decoded, ""} ->
        match_extended_schema(decoded, extended_schema)

      {:ok, wrong_data, rest} ->
        {:error, {:unmatched_data, wrong_data, rest}}

      {:error, error} ->
        {:error, error}
    end
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

  ## TODO: recursive tagged union: make it a part of bare.erl
  @spec to_bare_schema(extended_schema()) :: schema()
  def to_bare_schema(extended_schema) when is_list(extended_schema) do
    {:union, Keyword.values(extended_schema)}
  end

  def to_bare_schema(extended_schema) do
    extended_schema
  end
end

defmodule Ockam.Bare.Variant do
  @moduledoc """
  Support variant types
  Variant types are defined as {:variant, [atom() | {atom(), schema()]}
  The tag is encoded as bare enum, optionally followed by the field value in case
  the variant has one.
  """
  @type schema :: :bare.spec()
  @type extended_schema() :: schema() | {:variant, [atom() | {atom(), schema()}]}

  @spec encode(any(), extended_schema()) :: binary()
  def encode(value, {:variant, ss} = schema) do
    type = :bare.encode(enum_member(value), to_bare_schema(schema))
    value = encode_value(enum_value(value), List.keyfind(ss, enum_member(value), 0))
    <<type::binary, value::binary>>
  end

  def encode(value, schema), do: :bare.encode(value, schema)

  @spec decode(binary(), extended_schema()) :: {:ok, any()} | {:error, any()}
  def decode(data, {:variant, ss} = schema) do
    case :bare.decode(data, to_bare_schema(schema)) do
      {:ok, decoded, ""} ->
        {:ok, decoded}

      {:ok, decoded_tag, rest} ->
        {_, subschema} = List.keyfind(ss, decoded_tag, 0)

        with {:ok, decoded_value, ""} <- :bare.decode(rest, subschema) do
          {:ok, {decoded_tag, decoded_value}}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  def decode(data, schema), do: :bare.decode(data, schema)

  def to_bare_schema({:variant, ext_schema}) do
    {:enum, Enum.map(ext_schema, &enum_member/1)}
  end

  def to_bare_schema(schema), do: schema

  def enum_member({tag, _}), do: tag
  def enum_member(tag), do: tag

  def enum_value({_tag, value}), do: value
  def enum_value(_tag), do: nil

  def encode_value(nil, nil), do: <<>>
  def encode_value(value, {_tag, subschema}), do: :bare.encode(value, subschema)
end

defmodule Ockam.Bare.Extended do
  @moduledoc """
  Extension for BARE schema:

  Support simple tags for union types defined as [type1: schema(), type2: schema()] and
  variant defined as {:variant, [atom() | {atom(), schema()]}
  """

  alias Ockam.Bare.Union
  alias Ockam.Bare.Variant

  @type schema() :: any()
  @type extended_schema() ::
          schema() | [{atom(), schema()}] | {:variant, [atom() | {atom(), schema()}]}

  ## TODO: this might be moved to BARE lib
  def encode(data, {:variant, _} = schema), do: Variant.encode(data, schema)
  def encode(data, schema), do: Union.encode(data, schema)

  def decode(data, {:variant, _} = schema), do: Variant.decode(data, schema)
  def decode(data, schema), do: Union.decode(data, schema)
end
