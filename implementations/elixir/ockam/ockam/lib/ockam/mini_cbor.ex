defmodule MiniCBOR do
  @moduledoc """
    Wrapper for CBOR encoding library to work with values encoded using structures optimised by Rust
    https://twittner.gitlab.io/minicbor/minicbor_derive/index.html library

    Changes maps keys for encoded values to integers.
    Encodes atoms as index integers.

    Map keys optimization:

    Given data map `%{field1: 100, field2: "hi"}`
    and schema `{:map, [:field1, field2]}` // same as `{:map, [{:field1, :noschema}, {:field2, :noschema}]}`
    optimizes keys as `%{1 => 100, 2 => "hi"}`

    Enum atoms optimization:
    Given atom value `:other_thing`
    and schema `{:enum, [:one_thins, :other_thing, :another_thing]}`
    optimizes value as `1`

    Supports nested schemas in map key mapping:

    With data map `%{path: "/resource", method: :get}`
    and schema `{:map, [:path, {:method, {:enum, [:get, :post]}}`
    optimizes map as `%{1 => "/resource", 2 => 0}`

    When encoding a map or struct, the field `0` is reserved for use of type-tags (the tag feature is currently disabled on rust,
    and not implemented on elixir)
  """

  @type schema() :: {:map, [atom() | {atom(), schema()}]} | {:enum, [atom()]} | :noschema

  @reserved_tag_field :minicbor_tag_reserved

  @deprecated "Use Ockam.TypedCBOR instead"
  def encode(struct, schema) do
    schema_map = struct_schema(schema)
    optimized = rekey_struct(struct, schema_map)
    CBOR.encode(optimized)
  end

  @deprecated "Use Ockam.TypedCBOR instead"
  def decode(binary, schema) do
    with {:ok, optimized, rest} <- CBOR.decode(binary) do
      schema_map = optimized_schema(schema)
      struct = rekey_optimized(optimized, schema_map)
      {:ok, struct, rest}
    end
  end

  defp reserve_tag_field(keys) when is_list(keys) do
    # As a workaround, set this unused field at position 0.
    # Latter we will use position 0 to carry tag information.
    [@reserved_tag_field | keys]
  end

  def decode_strict(binary, schema) do
    case decode(binary, schema) do
      {:ok, decoded, ""} ->
        {:ok, decoded}

      {:ok, decoded, rest} ->
        {:error, {:decode_error, {:extra_data, rest, decoded}, binary}}

      {:error, _reason} = error ->
        error
    end
  end

  def struct_schema({:map, keys}) when is_list(keys) do
    mapping =
      reserve_tag_field(keys)
      |> Enum.with_index(fn
        {key, inner_schema}, index -> {key, {index, struct_schema(inner_schema)}}
        key, index -> {key, index}
      end)
      |> Map.new()

    {:map, mapping}
  end

  def struct_schema({:enum, options}) when is_list(options) do
    mapping =
      options
      |> Enum.with_index()
      |> Map.new()

    {:enum, mapping}
  end

  def struct_schema({:list, schema}) do
    {:list, struct_schema(schema)}
  end

  def optimized_schema({:map, keys}) when is_list(keys) do
    mapping =
      reserve_tag_field(keys)
      |> Enum.with_index(fn
        {key, inner_schema}, index -> {index, {key, optimized_schema(inner_schema)}}
        key, index -> {index, key}
      end)
      |> Map.new()

    {:map, mapping}
  end

  def optimized_schema({:enum, options}) when is_list(options) do
    mapping =
      options
      |> Enum.with_index(fn key, index -> {index, key} end)
      |> Map.new()

    {:enum, mapping}
  end

  def optimized_schema({:list, schema}) do
    {:list, optimized_schema(schema)}
  end

  def rekey_struct(struct, :noschema) do
    struct
  end

  def rekey_struct(struct, {:list, schema}) do
    Enum.map(struct, fn val ->
      rekey_struct(val, schema)
    end)
  end

  def rekey_struct(struct, {:map, schema_map}) do
    struct
    # because enum is not implemented for structs
    |> as_map()
    # Just in case
    |> Map.delete(@reserved_tag_field)
    |> Enum.flat_map(fn {key, val} ->
      case Map.get(schema_map, key) do
        nil ->
          []

        index when is_integer(index) ->
          [{index, val}]

        {index, inner_schema} when is_integer(index) ->
          [{index, rekey_struct(val, inner_schema)}]
      end
    end)
    |> Map.new()
  end

  def rekey_struct(atom, {:enum, option_map}) when is_atom(atom) do
    Map.fetch!(option_map, atom)
  end

  def rekey_optimized(optimized, :noschema) do
    optimized
  end

  def rekey_optimized(optimized, {:list, schema}) do
    Enum.map(optimized, fn val ->
      rekey_optimized(val, schema)
    end)
  end

  def rekey_optimized(optimized, {:map, schema_map}) do
    Enum.flat_map(optimized, fn {index, val} ->
      case Map.get(schema_map, index) do
        nil ->
          []

        {key, inner_schema} ->
          [{key, rekey_optimized(val, inner_schema)}]

        key ->
          [{key, val}]
      end
    end)
    |> Map.new()
  end

  def rekey_optimized(index, {:enum, option_map}) when is_integer(index) do
    Map.fetch!(option_map, index)
  end

  defp as_map(map) when is_struct(map) do
    Map.from_struct(map)
  end

  defp as_map(map) when is_map(map) do
    map
  end
end
