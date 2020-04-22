defprotocol Ockam.Router.Protocol.Encoding.Default.Encoder do
  @fallback_to_any true

  def encode(value, opts)
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: Any do
  alias Ockam.Router.Protocol.Encoding.Default.Encode
  alias Ockam.Router.Protocol.Encoding.Default.{Codegen, Encoder}

  defmacro __deriving__(module, struct, opts) do
    opts = Enum.into(opts, %{})
    schema = Map.fetch!(opts, :schema)
    keys = Enum.map(schema, fn {k, _} -> k end)
    fields = fields_to_encode(struct, keys)
    kv = Enum.map(fields, &{&1, generated_var(&1, __MODULE__)})
    encode_opts = quote(do: opts)
    encode_args = [encode_opts]

    iodata = Codegen.build_constant_iodata(kv, schema, encode_args)

    quote location: :keep do
      defimpl unquote(Encoder), for: unquote(module) do
        def encode(%{unquote_splicing(kv)}, unquote(encode_opts)) do
          {:ok, unquote(iodata)}
        end
      end
    end
  end

  # The same as Macro.var/2 except it sets generated: true
  defp generated_var(name, context) do
    {name, [generated: true], context}
  end

  defp fields_to_encode(%module{} = struct, keys) do
    known_keys =
      struct
      |> Map.keys()
      |> Enum.into(MapSet.new())

    expected_keys = Enum.into(keys, MapSet.new())

    unless MapSet.subset?(expected_keys, known_keys) do
      diff = MapSet.difference(expected_keys, known_keys)
      raise ArgumentError, message: "unknown fields for #{module}: #{inspect(diff)}"
    end

    keys
  end

  def encode(%_{} = struct, _opts) do
    raise Protocol.UndefinedError,
      protocol: @protocol,
      value: struct,
      description: """
      #{Encoder} protocol must always be explicitly implemented.

      If you own the struct, you can derive the implementation specifying \
      which fields should be encoded:

          @derive #{Encoder}, schema: [...]
          defstruct ...

      Finally, if you don't own the struct you want to encode, \
      you may use Protocol.derive/3 placed outside of any module:

          Protocol.derive(#{Encoder}, NameOfTheStruct, schema: [...])
      """
  end

  def encode(value, _opts) do
    raise Protocol.UndefinedError,
      protocol: @protocol,
      value: value,
      description: "#{Encoder} protocol must always be explicitly implemented"
  end
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: Atom do
  alias Ockam.Router.Protocol.Encoding.Default.Encode

  def encode(atom, opts) do
    {:ok, Encode.atom(atom, opts)}
  end
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: Integer do
  alias Ockam.Router.Protocol.Encoding.Default.Encode

  def encode(integer, opts) do
    {:ok, Encode.integer(integer, opts)}
  end
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: Float do
  alias Ockam.Router.Protocol.Encoding.Default.Encode

  def encode(float, opts) do
    {:ok, Encode.float(float, opts)}
  end
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: List do
  alias Ockam.Router.Protocol.Encoding.Default.Encode

  def encode(list, opts) do
    {:ok, Encode.list(list, opts)}
  end
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: Map do
  alias Ockam.Router.Protocol.Encoding.Default.Encode

  def encode(map, opts) do
    {:ok, Encode.map(map, opts)}
  end
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: BitString do
  alias Ockam.Router.Protocol.Encoding.Default.Encode

  def encode(binary, opts) when is_binary(binary) do
    {:ok, Encode.string(binary, opts)}
  end

  def encode(bitstring, opts) do
    {:ok, Encode.raw(bitstring, opts)}
  end
end

defimpl Ockam.Router.Protocol.Encoding.Default.Encoder, for: [NaiveDateTime, DateTime] do
  alias Ockam.Router.Protocol.Encoding.Default.Encode

  def encode(value, opts) do
    {:ok, Encode.iso8601_string(@for.to_iso8601(value), opts)}
  end
end
