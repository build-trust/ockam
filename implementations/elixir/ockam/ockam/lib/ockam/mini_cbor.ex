defmodule MiniCBOR do
  @moduledoc """
    Wrapper for CBOR encoding library to work with values encoded using structures optimised by Rust
    https://twittner.gitlab.io/minicbor/minicbor_derive/index.html library

    Changes maps keys for encoded values to integers.
    Encodes atoms as index integers.

    Map keys optimization:

    Given data map `%{field1: 100, field2: "hi"}`
    and schema `{:map, [:field1, field2]}` // same as `{:map, [{:field1, :noschema}, {:field2, :noschema}]}`
    optimizes keys as `%{0 => 100, 1 => "hi"}`

    Enum atoms optimization:
    Given atom value `:other_thing`
    and schema `{:enum, [:one_thins, :other_thing, :another_thing]}`
    optimizes value as `1`

    Supports nested schemas in map key mapping:

    With data map `%{path: "/resource", method: :get}`
    and schema `{:map, [:path, {:method, {:enum, [:get, :post]}}`
    optimizes map as `%{0 => "/resource", 1 => 0}`
  """

  @type schema() :: {:map, [atom() | {atom(), schema()}]} | {:enum, [atom()]} | :noschema

  def encode(struct, schema) do
    schema_map = struct_schema(schema)
    simple_map = rekey_struct(struct, schema_map)
    CBOR.encode(simple_map)
  end

  def decode(binary, schema) do
    with {:ok, simple_map, rest} <- CBOR.decode(binary) do
      schema_map = simple_map_schema(schema)
      struct = rekey_simple_map(simple_map, schema_map)
      {:ok, struct, rest}
    end
  end

  def struct_schema({:map, keys}) when is_list(keys) do
    mapping =
      keys
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

  def simple_map_schema({:map, keys}) when is_list(keys) do
    mapping =
      keys
      |> Enum.with_index(fn
        {key, inner_schema}, index -> {index, {key, simple_map_schema(inner_schema)}}
        key, index -> {index, key}
      end)
      |> Map.new()

    {:map, mapping}
  end

  def simple_map_schema({:enum, options}) when is_list(options) do
    mapping =
      options
      |> Enum.with_index(fn key, index -> {index, key} end)
      |> Map.new()

    {:enum, mapping}
  end

  def rekey_struct(struct, :noschema) do
    struct
  end

  def rekey_struct(struct, {:map, schema_map}) do
    struct
    # because enum is not implemented for structs
    |> Map.from_struct()
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

  def rekey_simple_map(simple_map, :noschema) do
    simple_map
  end

  def rekey_simple_map(simple_map, {:map, schema_map}) do
    Enum.flat_map(simple_map, fn {index, val} ->
      case Map.get(schema_map, index) do
        nil ->
          []

        {key, inner_schema} ->
          [{key, rekey_simple_map(val, inner_schema)}]

        key ->
          [{key, val}]
      end
    end)
    |> Map.new()
  end

  def rekey_simple_map(index, {:enum, option_map}) when is_integer(index) do
    Map.fetch!(option_map, index)
  end
end
