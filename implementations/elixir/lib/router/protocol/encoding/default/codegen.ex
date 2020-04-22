defmodule Ockam.Router.Protocol.Encoding.Default.Codegen do
  alias Ockam.Router.Protocol.Encoding.Default.Encode
  alias Ockam.Router.Protocol.Encoding.Default.Decode

  def build_kv_iodata(kv, encode_opts, encode_args) do
    kv
    |> Enum.map(&encode_pair(&1, encode_opts, encode_args))
    |> List.flatten()
    |> collapse_static()
  end

  def build_constant_iodata(kv, schema, encode_args) do
    Enum.map(kv, &encode_value(&1, schema, encode_args))
  end

  def build_decoder(_module, value, input, [], _decode_args) do
    quote(do: {:ok, unquote(value), unquote(input)})
  end

  def build_decoder(module, value, input, schema, decode_args) do
    decodes = build_decoder(module, value, input, schema, decode_args, [])

    quote do
      with unquote_splicing(decodes) do
        {:ok, unquote(value), unquote(input)}
      end
    end
  end

  defp build_decoder(_module, _value, _input, [], _decode_args, acc), do: Enum.reverse(acc)

  defp build_decoder(module, value, input, [{key, {type, default}} | schema], decode_args, acc) do
    acc = build_value_decoder(module, value, input, key, type, default, decode_args, acc)
    build_decoder(module, value, input, schema, decode_args, acc)
  end

  defp build_decoder(module, value, input, [{key, type} | schema], decode_args, acc) do
    acc = build_value_decoder(module, value, input, key, type, nil, decode_args, acc)
    build_decoder(module, value, input, schema, decode_args, acc)
  end

  defp build_value_decoder(_module, value, input, key, type, _default, decode_args, acc) do
    binding = {key, [generated: true], __MODULE__}

    bound =
      quote do
        {:ok, unquote(binding), unquote(input)} <-
          unquote(Decode).decode(
            unquote(type),
            unquote_splicing(decode_args)
          )
      end

    applied =
      quote do
        unquote(value) = Map.put(unquote(value), unquote(key), unquote(binding))
      end

    [applied, bound | acc]
  end

  defp encode_pair({key, value}, encode_opts, encode_args) do
    key = IO.iodata_to_binary(Encode.atom(key, encode_opts))
    [key, quote(do: unquote(Encode).encode_value(unquote(value), unquote_splicing(encode_args)))]
  end

  defp encode_value({key, value}, schema, encode_args) do
    type = Keyword.fetch!(schema, key)

    case type do
      :i1 ->
        quote(do: {:ok, unquote(Encode).i1(unquote(value), unquote_splicing(encode_args))})

      _other ->
        quote(do: unquote(Encode).encode_value(unquote(value), unquote_splicing(encode_args)))
    end
  end

  defp collapse_static([bin1, bin2 | rest]) when is_binary(bin1) and is_binary(bin2) do
    collapse_static([bin1 <> bin2 | rest])
  end

  defp collapse_static([other | rest]) do
    [other | collapse_static(rest)]
  end

  defp collapse_static([]) do
    []
  end
end
