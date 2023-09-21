defmodule Ockam.TypedCBOR do
  @moduledoc """
  Helpers encode/decode structs to/from CBOR,  aimed at compatibility with minicbor rust library.
  Preferred usage is through TypedStruct macros, see examples on test/plugin_test.exs
  """

  @doc ~S"""
      iex> to_cbor_term(:integer, 2)
      2
      iex> from_cbor_term(:integer, to_cbor_term(:integer, 2))
      2

      iex> to_cbor_term(:integer, "2")
      ** (Ockam.TypedCBOR.Exception) type mismatch, expected schema :integer

      iex> to_cbor_term(:string, "2")
      "2"

      iex> from_cbor_term(:string, to_cbor_term(:string, "2"))
      "2"

      iex> from_cbor_term(:integer, to_cbor_term(:string, "2"))
      ** (Ockam.TypedCBOR.Exception) type mismatch, expected schema :integer

      iex> to_cbor_term(:string, <<255>>)
      ** (Ockam.TypedCBOR.Exception) invalid string <<255>>

      iex> from_cbor_term(:string, <<255>>)
      ** (Ockam.TypedCBOR.Exception) invalid string <<255>>

      iex> to_cbor_term(:boolean, true)
      true

      iex> from_cbor_term(:boolean, to_cbor_term(:boolean, true))
      true

      iex> to_cbor_term(:binary, <<255>>)
      %CBOR.Tag{tag: :bytes, value: <<255>>}

      iex> from_cbor_term(:binary, to_cbor_term(:binary, <<255>>))
      <<255>>

      iex> to_cbor_term({:enum, [a: 0, b: 1]}, :a)
      0

      iex> from_cbor_term({:enum, [a: 0, b: 1]}, to_cbor_term({:enum, [a: 0, b: 1]}, :b))
      :b

      iex> to_cbor_term({:enum, [a: 0, b: 1]}, :c)
      ** (Ockam.TypedCBOR.Exception) invalid enum val: :c, allowed: [:a, :b]

      iex> from_cbor_term({:enum, [a: 0, b: 1]}, 3)
      ** (Ockam.TypedCBOR.Exception) invalid enum encoding: 3, allowed: [a: 0, b: 1]

      iex> to_cbor_term({:list, :integer}, [0, 1])
      [0,1]

      iex> from_cbor_term({:list, :integer}, to_cbor_term({:list, :integer}, [0, 1]))
      [0,1]

      iex> to_cbor_term({:list, {:enum, [a: 0, b: 1]}}, [:b, :c])
      ** (Ockam.TypedCBOR.Exception) invalid enum val: :c, allowed: [:a, :b]

      iex> to_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                          a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>              %{a1: "aa", a2: nil})
      %{1 => "aa"}

      iex> from_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                            a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>                 to_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                                         a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>                              %{a1: "aa", a2: nil}))
      %{a1: "aa"}

      iex> to_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                                a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>                    %{a1: "aa", a2: :a})
      %{1 => "aa", 2 => 0}

      iex> from_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                            a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>                 to_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                                         a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>                              %{a1: "aa", a2: :a}))
      %{a1: "aa", a2: :a}


      iex> to_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                          a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}},
      ...>                          a3: %{key: 3, schema: {:list, :binary}}}},
      ...>              %{a1: "aa", a2: :a, a3: ["hello", "world"]})
      %{1 => "aa", 2 => 0, 3 => [%CBOR.Tag{tag: :bytes, value: "hello"}, %CBOR.Tag{tag: :bytes, value: "world"}]}


      iex> to_cbor_term({:struct, %{a1: %{key: 1, schema: :string, required: true},
      ...>                          a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>               %{a2: :b})
      ** (Ockam.TypedCBOR.Exception) field a1 is required

      iex> from_cbor_term({:struct, %{tag: %{key: 0, schema: :integer, required: true, constant: 123},
      ...>                            a1: %{key: 1, schema: :string, required: true},
      ...>                            a2: %{key: 2, schema: {:enum, [a: 0, b: 1]}}}},
      ...>                %{0 => 123, 2 => 0})
      ** (Ockam.TypedCBOR.Exception) field a1 (1) is required
  """

  alias Ockam.TypedCBOR.Exception

  require Logger

  def from_cbor_term(:integer, val) when is_integer(val), do: val

  def from_cbor_term(:boolean, val) when is_boolean(val), do: val

  def from_cbor_term(:string, val) when is_binary(val) do
    if String.valid?(val) do
      val
    else
      raise Exception, message: "invalid string #{inspect(val)}"
    end
  end

  def from_cbor_term(:binary, %CBOR.Tag{tag: :bytes, value: val}), do: val

  def from_cbor_term({:enum, vals}, n) when is_integer(n) do
    case List.keyfind(vals, n, 1) do
      nil ->
        raise Exception, message: "invalid enum encoding: #{n}, allowed: #{inspect(vals)}"

      {val, ^n} ->
        val
    end
  end

  def from_cbor_term({:list, element_schema}, values) when is_list(values),
    do: Enum.map(values, fn val -> from_cbor_term(element_schema, val) end)

  def from_cbor_term({:map, key_schema, value_schema}, values) when is_map(values) do
    values
    |> Enum.map(fn {k, v} -> {from_cbor_term(key_schema, k), from_cbor_term(value_schema, v)} end)
    |> Enum.into(%{})
  end

  def from_cbor_term({:struct, fields}, struct) when is_map(struct) do
    from_cbor_fields(Map.to_list(fields), struct) |> Enum.into(%{})
  end

  def from_cbor_term({:struct, mod, fields}, struct) when is_map(struct) do
    struct(mod, from_cbor_term({:struct, fields}, struct))
  end

  def from_cbor_term(:term, any), do: any

  def from_cbor_term(schema, data) do
    with true <- is_atom(schema),
         true <- function_exported?(schema, :from_cbor_term, 1),
         {:ok, val} <- schema.from_cbor_term(data) do
      val
    else
      _ ->
        Logger.error("type mismatch, expected schema #{inspect(schema)}, value: #{inspect(data)}")
        raise(Exception, "type mismatch, expected schema #{inspect(schema)}")
    end
  end

  def from_cbor_fields([], _), do: []

  def from_cbor_fields([{field_name, field_options} | rest], struct) do
    key = Map.fetch!(field_options, :key)
    schema = Map.fetch!(field_options, :schema)
    required = Map.get(field_options, :required, false)

    case Map.fetch(struct, key) do
      :error when required ->
        raise Exception, message: "field #{field_name} (#{key}) is required"

      :error ->
        from_cbor_fields(rest, struct)

      {:ok, val} ->
        val = from_cbor_term(schema, val)

        case Map.fetch(field_options, :constant) do
          :error ->
            [{field_name, val} | from_cbor_fields(rest, struct)]

          {:ok, ^val} ->
            [{field_name, val} | from_cbor_fields(rest, struct)]

          {:ok, expected} ->
            raise Exception,
              message: "field #{field_name} expected #{inspect(expected)}, got #{inspect(val)}"
        end
    end
  end

  def to_cbor_term(:integer, val) when is_integer(val), do: val

  def to_cbor_term(:boolean, val) when is_boolean(val), do: val

  def to_cbor_term(:string, val) when is_binary(val) do
    if String.valid?(val) do
      val
    else
      raise Exception, message: "invalid string #{inspect(val)}"
    end
  end

  def to_cbor_term(:binary, val) when is_binary(val), do: %CBOR.Tag{tag: :bytes, value: val}

  def to_cbor_term({:enum, vals}, val) when is_atom(val) do
    case vals[val] do
      nil ->
        raise Exception,
          message: "invalid enum val: #{inspect(val)}, allowed: #{inspect(Keyword.keys(vals))}"

      n when is_integer(n) ->
        n
    end
  end

  def to_cbor_term({:list, element_schema}, values) when is_list(values),
    do: Enum.map(values, fn val -> to_cbor_term(element_schema, val) end)

  def to_cbor_term({:map, key_schema, value_schema}, values) when is_map(values) do
    values
    |> Enum.map(fn {k, v} -> {to_cbor_term(key_schema, k), to_cbor_term(value_schema, v)} end)
    |> Enum.into(%{})
  end

  def to_cbor_term({:struct, fields}, struct) when is_map(struct) do
    extra_keys = Map.drop(struct, Map.keys(fields))

    if not Enum.empty?(extra_keys),
      do: raise(Exception, message: "Extra fields: #{inspect(Map.keys(extra_keys))}")

    to_cbor_fields(Map.to_list(fields), struct) |> Enum.into(%{})
  end

  def to_cbor_term({:struct, mod, fields}, struct) when is_map(struct) do
    if struct.__struct__ != mod,
      do: raise(Exception, message: "a %#{mod}{} struct must be provided")

    to_cbor_fields(Map.to_list(fields), struct) |> Enum.into(%{})
  end

  def to_cbor_term(:term, any), do: any

  def to_cbor_term(schema, val) do
    with true <- is_atom(schema),
         true <- function_exported?(schema, :to_cbor_term, 1),
         {:ok, cbor} <- schema.to_cbor_term(val) do
      cbor
    else
      _ ->
        Logger.error("type mismatch, expected schema #{inspect(schema)}, value: #{inspect(val)}")
        raise(Exception, "type mismatch, expected schema #{inspect(schema)}")
    end
  end

  def to_cbor_fields([], _), do: []

  def to_cbor_fields([{field_name, field_options} | rest], struct) do
    key = Map.fetch!(field_options, :key)
    schema = Map.fetch!(field_options, :schema)
    required = Map.get(field_options, :required, false)

    case Map.get(struct, field_name) do
      nil when required ->
        raise Exception, message: "field #{field_name} is required"

      nil ->
        to_cbor_fields(rest, struct)

      val ->
        [{key, to_cbor_term(schema, val)} | to_cbor_fields(rest, struct)]
    end
  end

  def encode!(schema, d),
    do: CBOR.encode(to_cbor_term(schema, d))

  def encode(schema, d) do
    case wrap_exception(&encode!(schema, &1), d) do
      data when is_binary(data) -> {:ok, data}
      {:error, _reason} = error -> error
    end
  end

  def decode!(schema, data) do
    with {:ok, map, rest} <- CBOR.decode(data) do
      {:ok, from_cbor_term(schema, map), rest}
    end
  end

  def decode(schema, data), do: wrap_exception(&decode!(schema, &1), data)

  def decode_strict(schema, data) do
    case decode(schema, data) do
      {:ok, decoded, ""} -> {:ok, decoded}
      {:ok, decoded, rest} -> {:error, {:decode_error, {:extra_data, rest, decoded}, data}}
      {:error, _reason} = error -> error
    end
  end

  defp wrap_exception(f, arg) do
    f.(arg)
  rescue
    e in Ockam.TypedCBOR.Exception ->
      {:error, e.message}
  end
end
