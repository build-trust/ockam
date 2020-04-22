defprotocol Ockam.Router.Protocol.Encoding.Default.Decoder do
  @fallback_to_any true

  def decode(value, encoded, opts)
end

defimpl Ockam.Router.Protocol.Encoding.Default.Decoder, for: Any do
  alias Ockam.Router.Protocol.Encoding.Default.{Codegen, Decoder, Decode}

  defmacro __deriving__(module, struct, opts) do
    schema = Keyword.fetch!(opts, :schema)
    value = generated_var(:value, __MODULE__)
    input = generated_var(:input, __MODULE__)
    keys_fun = generated_var(:keys_fun, __MODULE__)
    strings_fun = generated_var(:strings_fun, __MODULE__)
    decode_opts = quote(do: opts)
    decode_args = [input, keys_fun, strings_fun, decode_opts]

    fieldless = 0 == struct |> Map.from_struct() |> map_size()
    decoder = Codegen.build_decoder(module, value, input, schema, decode_args)

    if fieldless do
      quote location: :keep do
        defimpl unquote(Decoder), for: unquote(module) do
          def decode(unquote(value), unquote(input), opts) do
            unquote(decoder)
          end
        end
      end
    else
      quote location: :keep do
        defimpl unquote(Decoder), for: unquote(module) do
          def decode(unquote(value), unquote(input), opts) do
            unquote(keys_fun) = unquote(Decode).key_decoder(opts)
            unquote(strings_fun) = unquote(Decode).string_decoder(opts)
            unquote(decoder)
          end
        end
      end
    end
  end

  # The same as Macro.var/2 except it sets generated: true
  defp generated_var(name, context) do
    {name, [generated: true], context}
  end

  def decode(%_{} = struct, _encoded, _opts) do
    raise Protocol.UndefinedError,
      protocol: @protocol,
      value: struct,
      description: """
      #{Decoder} protocol must always be explicitly implemented.

      If you own the struct, you can derive the implementation specifying \
      which fields should be encoded:

          @derive {#{Decoder}, schema: [...]
          defstruct ...

      Finally, if you don't own the struct you want to encode, \
      you may use Protocol.derive/3 placed outside of any module:

          Protocol.derive(#{Decoder}, NameOfTheStruct, schema: [...])
      """
  end

  def decode(value, _encoded, _opts) do
    raise Protocol.UndefinedError,
      protocol: @protocol,
      value: value,
      description: "#{Decoder} protocol must always be explicitly implemented"
  end
end
