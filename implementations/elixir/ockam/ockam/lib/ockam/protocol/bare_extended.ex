defmodule Ockam.Bare.Extended do
  @moduledoc """
  Extension for BARE schema to support simple tags for union types.

  Union type can be defined as [type1: schema, type2: schema]
  and can be encoded and decoded from/to {:type1, data} or {:type2, data}
  """

  @type schema() :: any()
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

      other ->
        other
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

  @spec to_bare_schema(extended_schema()) :: schema()
  def to_bare_schema(extended_schema) when is_list(extended_schema) do
    {:union, Keyword.values(extended_schema)}
  end

  def to_bare_schema(extended_schema) do
    extended_schema
  end
end
