defmodule Ockam.Router.Protocol.EncodeError do
  defexception [:message]

  @type t :: %__MODULE__{message: String.t()}

  def new({:invalid_leb128, v}) do
    %__MODULE__{message: "value cannot be encoded as an LEB128 integer: `#{inspect(v)}`"}
  end
end

defmodule Ockam.Router.Protocol.DecodeError do
  defexception [:message]

  @type t :: %__MODULE__{message: String.t()}

  def new({reason, bytes}) when reason in [:invalid_leb128, :invalid_leb128_u2] do
    %__MODULE__{
      message: "expected an LEB128-encoded integer, but got: #{inspect(bytes, base: :hex)}"
    }
  end

  def new({:invalid_i1, bytes}) do
    %__MODULE__{
      message: "expected an i1-encoded integer, but got: #{inspect(bytes, base: :hex)}"
    }
  end

  def new({:unexpected_eof, expected, got}) do
    %__MODULE__{
      message: "unexpected EOF: expected to decode #{expected} bytes, but only found #{got}"
    }
  end

  def new({:invalid_message_type, type}) do
    %__MODULE__{
      message: "unrecognized message type code #{inspect(type, base: :hex)}"
    }
  end

  def new({:invalid_message_body, type, body}) do
    %__MODULE__{
      message: "unrecognized message content for type #{inspect(type)}: #{inspect(body)}"
    }
  end

  def new({:type_error, {type, raw}}) do
    %__MODULE__{
      message: "unable to decode value of type #{type}, got: #{inspect(raw, base: :hex)}"
    }
  end

  def new({:type_error, {type, raw, reason}}) do
    %__MODULE__{
      message:
        "unable to decode value of type #{type} (#{inspect(reason)}), got: #{
          inspect(raw, base: :hex)
        }"
    }
  end

  def new({:invalid_type, type}) do
    %__MODULE__{
      message: "unrecognized type code #{inspect(type, base: :hex)}"
    }
  end

  def new(message) when is_binary(message) do
    %__MODULE__{message: message}
  end
end
