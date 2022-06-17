defmodule Ockam.TypedCBOR.Plugin do
  @moduledoc """
  TypedStruct plugin to add minicbor type-based encoding/decoding
  """

  use TypedStruct.Plugin

  @impl true
  defmacro init(_) do
    quote do
      Module.register_attribute(__MODULE__, :tt_fields, accumulate: true)
    end
  end

  @impl true
  def field(name, type, [minicbor: minicbor], env) do
    %Macro.Env{module: mod} = env
    key = Keyword.fetch!(minicbor, :key)
    schema = field_schema(minicbor[:schema], type_to_spec(type))
    Module.put_attribute(mod, :tt_fields, {name, Map.put(schema, :key, key)})
  end

  def field(name, _, options, _),
    do:
      raise(
        "field #{name} must supply a :minicbor option, and no other option is allowed #{inspect(options)}"
      )

  defp field_schema(nil, [nil, t]), do: %{schema: t, required: false}
  defp field_schema(nil, [t, nil]), do: %{schema: t, required: false}

  defp field_schema(nil, t) when is_list(t),
    do: raise("enum type #{inspect(t)} must specify a schema")

  defp field_schema(nil, t), do: %{schema: t, required: true}
  defp field_schema(schema, [_, nil]), do: %{schema: schema, required: false}
  defp field_schema(schema, [nil, _]), do: %{schema: schema, required: false}

  defp field_schema({:enum, mappings}, l) when is_list(l) do
    if Enum.sort(Keyword.keys(mappings)) != Enum.sort(l) do
      raise("schema #{inspect(mappings)} must provide mapping for enum type #{inspect(l)}")
    end

    %{schema: {:enum, mappings}, required: true}
  end

  defp field_schema(schema, t) when is_list(t),
    do: raise("provider schema #{inspect(schema)} must match enum type #{inspect(t)}")

  defp field_schema(schema, _), do: %{schema: schema, required: true}

  def type_to_spec({:binary, _, _}), do: :binary

  def type_to_spec({:string, _, _}),
    do:
      raise(
        "string() type not supported, use either String.t() for utf8 strings,  or binary() for raw data"
      )

  def type_to_spec({:integer, _, _}), do: :integer
  def type_to_spec({:boolean, _, _}), do: :boolean
  def type_to_spec({:list, _, [child]}), do: {:list, type_to_spec(child)}
  def type_to_spec({:%{}, _, [{key, val}]}), do: {:map, type_to_spec(key), type_to_spec(val)}
  def type_to_spec({:map, _, []}), do: {:map, :term, :term}
  def type_to_spec({:|, _, _} = options), do: extract_options(options)
  def type_to_spec({{:., _, [{:__aliases__, _, [:String]}, :t]}, _, _}), do: :string
  def type_to_spec({:term, _, _}), do: :term
  def type_to_spec(val), do: val

  def extract_option(val), do: type_to_spec(val)
  def extract_options({:|, _, [opt1, opts2]}), do: [extract_option(opt1) | extract_options(opts2)]
  def extract_options(val), do: [extract_option(val)]

  @impl true
  def after_definition(_) do
    quote do
      def minicbor_schema(), do: {:struct, __MODULE__, @tt_fields |> Enum.into(%{})}

      def encode!(%__MODULE__{} = d), do: Ockam.TypedCBOR.encode!(minicbor_schema(), d)

      def encode(%__MODULE__{} = d), do: Ockam.TypedCBOR.encode(minicbor_schema(), d)

      def decode!(data), do: Ockam.TypedCBOR.decode!(minicbor_schema(), data)

      def decode(data), do: Ockam.TypedCBOR.decode(minicbor_schema(), data)

      def decode_strict(data), do: Ockam.TypedCBOR.decode_strict(minicbor_schema(), data)

      def encode_list!(l), do: Ockam.TypedCBOR.encode!({:list, minicbor_schema()}, l)

      def encode_list(l), do: Ockam.TypedCBOR.encode({:list, minicbor_schema()}, l)

      def decode_list!(data), do: Ockam.TypedCBOR.decode!({:list, minicbor_schema()}, data)

      def decode_list(data), do: Ockam.TypedCBOR.decode({:list, minicbor_schema()}, data)
    end
  end
end
