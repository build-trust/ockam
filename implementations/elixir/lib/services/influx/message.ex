defmodule Ockam.Services.Influx.Message do
  defstruct [:value]

  defmodule Write do
    defstruct [:measurement, :tags, :fields]

    def type_id(%__MODULE__{}), do: 0

    defimpl Ockam.Router.Protocol.Encoder do
      def encode(value, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Router.Protocol.Encoding.Default.Encode
      alias Ockam.Services.Influx.Message.Write

      def encode(%Write{measurement: measurement, tags: tags, fields: fields} = value, opts) do
        type = Encode.i1(Write.type_id(value), opts)
        measure_len = Helpers.encode_leb128_u2(byte_size(measurement))
        tags_len = Helpers.encode_leb128_u2(length(tags))

        tags_encoded =
          for {name, value} <- tags do
            name_str = to_string(name)
            value_str = to_string(value)
            name_len = Helpers.encode_leb128_u2(byte_size(name_str))
            value_len = Helpers.encode_leb128_u2(byte_size(value_str))
            [name_len, name_str, value_len, value_str]
          end

        fields_len = Helpers.encode_leb128_u2(length(fields))

        fields_encoded =
          for {name, value} <- fields do
            name_str = to_string(name)
            name_len = Helpers.encode_leb128_u2(byte_size(name_str))
            value_type = encode_value_type(value)
            value_encoded = encode_value(value)
            [name_len, name_str, value_type, value_encoded]
          end

        {:ok,
         [type, measure_len, measurement, tags_len, tags_encoded, fields_len, fields_encoded]}
      end

      defp encode_value_type(n) when is_number(n), do: 0
      defp encode_value_type(true), do: 1
      defp encode_value_type(false), do: 2
      defp encode_value_type(_), do: 3

      defp encode_value(n) when is_number(n), do: <<n::big-size(8)-unit(8)>>
      defp encode_value(b) when is_boolean(b), do: <<>>

      defp encode_value(s) when is_binary(s) do
        [Helpers.encode_leb128_u2(byte_size(s)), s]
      end

      defp encode_value(value) do
        value_str = to_string(value)
        [Helpers.encode_leb128_u2(byte_size(value_str)), value_str]
      end
    end

    defimpl Ockam.Router.Protocol.Decoder do
      def decode(value, input, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Services.Influx.Message.Write

      def decode(value, input, _opts) do
        with {:ok, measurement, rest} <- decode_measurement(input),
             {:ok, tags, rest} <- decode_tags(rest),
             {:ok, fields, rest} <- decode_fields(rest) do
          {:ok, %Write{value | measurement: measurement, tags: tags, fields: fields}, rest}
        end
      end

      defp decode_measurement(input) do
        {measure_len, rest} = Helpers.decode_leb128_u2(input)

        if measure_len > 0 do
          <<measurement::binary-size(measure_len), rest::binary>> = rest
          {:ok, measurement, rest}
        else
          {:error, :invalid_measurement_name}
        end
      end

      defp decode_tags(input) do
        {tags_len, rest} = Helpers.decode_leb128_u2(input)
        do_decode_tags(tags_len, rest, [])
      end

      defp do_decode_tags(0, rest, acc), do: {:ok, acc, rest}

      defp do_decode_tags(n, rest, acc) do
        {name_len, rest} = Helpers.decode_leb128_u2(rest)

        if name_len > 0 do
          <<name_str::binary-size(name_len), rest::binary>> = rest
          {value_len, rest} = Helpers.decode_leb128_u2(rest)

          if value_len > 0 do
            <<value_str::binary-size(value_len), rest::binary>> = rest
            do_decode_tags(n - 1, rest, [{name_str, value_str} | acc])
          else
            do_decode_tags(n - 1, rest, [{name_str, nil} | acc])
          end
        else
          {:error, :invalid_tag_name}
        end
      end

      defp decode_fields(input) do
        {fields_len, rest} = Helpers.decode_leb128_u2(input)

        if fields_len > 0 do
          do_decode_fields(fields_len, rest, [])
        else
          {:error, :invalid_empty_fields}
        end
      end

      defp do_decode_fields(0, rest, acc), do: {:ok, acc, rest}

      defp do_decode_fields(n, rest, acc) do
        {name_len, rest} = Helpers.decode_leb128_u2(rest)

        if name_len > 0 do
          <<name_str::binary-size(name_len), type_id::8, rest::binary>> = rest

          case type_id do
            0 ->
              <<value::big-size(8)-unit(8), rest::binary>> = rest
              do_decode_fields(n - 1, rest, [{name_str, value} | acc])

            1 ->
              do_decode_fields(n - 1, rest, [{name_str, true} | acc])

            2 ->
              do_decode_fields(n - 1, rest, [{name_str, false} | acc])

            3 ->
              {value_len, rest} = Helpers.decode_leb128_u2(rest)
              <<value_str::binary-size(value_len), rest::binary>> = rest
              do_decode_fields(n - 1, rest, [{name_str, value_str} | acc])

            _ ->
              {:error, {:invalid_type_id, type_id}}
          end
        else
          {:error, :invalid_field_name}
        end
      end
    end
  end

  defmodule Query do
    defstruct [:text]

    def type_id(%__MODULE__{}), do: 1

    defimpl Ockam.Router.Protocol.Encoder do
      def encode(value, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
      alias Ockam.Router.Protocol.Encoding.Default.Encode
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Services.Influx.Message.Query

      def encode(%Query{text: text} = value, opts) when is_binary(text) do
        type = Encode.i1(Query.type_id(value), opts)
        len = Helpers.encode_leb128_u2(byte_size(text))
        {:ok, [type, len, text]}
      end
    end

    defimpl Ockam.Router.Protocol.Decoder do
      def decode(value, input, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
      alias Ockam.Router.Protocol.DecodeError
      alias Ockam.Router.Protocol.Encoding.Default.Decoder
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Services.Influx.Message.Query

      def decode(value, input, _opts) do
        {len, rest} = Helpers.decode_leb128_u2(input)

        if len > 0 do
          <<data::binary-size(len), rest::binary>> = rest
          {:ok, %Query{value | text: data}, rest}
        else
          {:ok, value, rest}
        end
      end
    end
  end

  def write(measurement, tags, fields) do
    %__MODULE__{value: %Write{measurement: measurement, tags: tags, fields: fields}}
  end

  def query(text) do
    %__MODULE__{value: %Query{text: text}}
  end

  def value(%__MODULE__{value: value}), do: value

  defimpl Ockam.Router.Protocol.Encoder do
    def encode(value, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
    alias Ockam.Services.Influx.Message

    def encode(%Message{value: value}, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
  end

  defimpl Ockam.Router.Protocol.Decoder do
    def decode(value, input, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
    alias Ockam.Router.Protocol.DecodeError
    alias Ockam.Router.Protocol.Encoding.Default.Decoder
    alias Ockam.Services.Influx.Message

    def decode(value, <<type::8, input::binary>>, opts) do
      with {:ok, message_type_mod, opts} <- message_type(type, opts),
           {:ok, message_type, rest} <- decode_message_type(message_type_mod, input, opts) do
        {:ok, %Message{value | value: message_type}, rest}
      end
    end

    defp decode_message_type(mod, input, opts) do
      Decoder.decode(struct(mod, []), input, opts)
    end

    defp message_type(0, opts), do: {:ok, Message.Write, opts}
    defp message_type(1, opts), do: {:ok, Message.Query, opts}
    defp message_type(n, _opts), do: {:error, DecodeError.new({__MODULE__, {:invalid_type, n}})}
  end
end
