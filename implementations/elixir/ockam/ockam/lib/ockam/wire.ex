defmodule Ockam.Wire do
  @moduledoc """
  Encodes and decodes messages that can be transported on the wire.
  """

  alias Ockam.Address
  alias Ockam.Message

  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  @default_implementation Ockam.Wire.Binary.V1

  @doc """
  Encodes a message into a binary.

  Returns `{:ok, iodata}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback encode(message :: Message.t()) ::
              {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}

  @doc """
  Encodes a route into a binary.

  Returns `{:ok, iodata}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback encode_route(route :: Address.route()) ::
              {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}

  @doc """
  Encodes an address into a binary.

  Returns `{:ok, iodata}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback encode_address(address :: Address.t()) ::
              {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}

  @doc """
  Decodes a message from a binary.

  Returns `{:ok, message}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback decode(encoded :: binary()) ::
              {:ok, message :: Message.t()} | {:error, error :: DecodeError.t()}

  @doc """
  Decodes a route from a binary.

  Returns `{:ok, message}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback decode_route(encoded :: binary()) ::
              {:ok, route :: Address.route()} | {:error, error :: DecodeError.t()}

  @doc """
  Decodes an address from a binary.

  Returns `{:ok, message}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback decode_address(encoded :: binary()) ::
              {:ok, address :: Address.t()} | {:error, error :: DecodeError.t()}

  @doc """
  Encode a message to a binary using the provided encoder.
  """
  @spec encode(message :: Message.t(), encoder :: atom) ::
          {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}

  def encode(message, encoder \\ nil) do
    with_implementation(encoder, :encode, [message])
  end

  @doc """
  Encode a route to a binary using the provided encoder.
  """
  @spec encode_route(route :: Address.route(), encoder :: atom) ::
          {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}
  def encode_route(route, encoder \\ nil) do
    with_implementation(encoder, :encode_route, [route])
  end

  @doc """
  Encode an address to a binary using the provided encoder.
  """
  @spec encode_address(message :: Address.t(), encoder :: atom) ::
          {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}
  def encode_address(address, encoder \\ nil) do
    with_implementation(encoder, :encode_address, [address])
  end

  @doc """
  Decode a message from binary using the provided decoder.
  Sets local metadata :channel key to channel
  """
  @spec decode(encoded :: binary(), channel :: atom(), decoder :: atom()) ::
          {:ok, message :: Message.t()} | {:error, reason :: DecodeError.t()}

  def decode(encoded, channel \\ :unknown, decoder \\ nil)

  def decode(encoded, channel, decoder) when is_binary(encoded) do
    with {:ok, message} <- with_implementation(decoder, :decode, [encoded]) do
      metadata = %{source: :channel, channel: channel}
      {:ok, Message.set_local_metadata(message, metadata)}
    end
  end

  def decode(encoded, _channel, _decoder) do
    {:error, error(:decode, {:encoded_input_is_not_binary, encoded})}
  end

  @doc """
  Decode a route from binary using the provided decoder.
  """
  @spec decode_route(encoded :: binary, decoder :: atom) ::
          {:ok, route :: Address.route()} | {:error, error :: DecodeError.t()}

  def decode_route(encoded, decoder \\ nil)

  def decode_route(encoded, decoder) when is_binary(encoded) do
    with_implementation(decoder, :decode_route, [encoded])
  end

  def decode_route(encoded, _decoder) do
    {:error, error(:decode_route, {:encoded_input_is_not_binary, encoded})}
  end

  @doc """
  Decode an address from binary using the provided decoder.
  """
  @spec decode_address(encoded :: binary, decoder :: atom) ::
          {:ok, address :: Address.t()} | {:error, error :: DecodeError.t()}

  def decode_address(encoded, decoder \\ nil)

  def decode_address(encoded, decoder) when is_binary(encoded) do
    with_implementation(decoder, :decode_address, [encoded])
  end

  def decode_address(encoded, _decoder) do
    {:error, error(:decode_address, {:encoded_input_is_not_binary, encoded})}
  end

  def with_implementation(nil, fun_name, args) do
    case default_implementation() do
      nil ->
        error(fun_name, :no_default_implementation)

      module when is_atom(module) ->
        with_implementation(module, fun_name, args)

      other ->
        error(fun_name, {:implementation_is_not_a_module, other})
    end
  end

  def with_implementation(module, fun_name, args) when is_atom(module) do
    with :ok <- ensure_loaded(fun_name, module),
         :ok <- ensure_exported(module, fun_name, Enum.count(args)) do
      apply(module, fun_name, args)
    else
      {:error, reason} -> error(fun_name, reason)
    end
  end

  def error(:encode, reason) do
    {:error, EncodeError.new(reason)}
  end

  def error(:encode_route, reason) do
    {:error, EncodeError.new(reason)}
  end

  def error(:encode_address, reason) do
    {:error, EncodeError.new(reason)}
  end

  def error(:decode, reason) do
    {:error, DecodeError.new(reason)}
  end

  def error(:decode_route, reason) do
    {:error, DecodeError.new(reason)}
  end

  def error(:decode_address, reason) do
    {:error, DecodeError.new(reason)}
  end

  # returns :ok if module is loaded, {:error, reason} otherwise
  defp ensure_loaded(type, module) do
    case Code.ensure_loaded?(module) do
      true -> :ok
      false -> {:error, {:module_not_loaded, {type, module}}}
    end
  end

  # returns :ok if a module exports the given function, {:error, reason} otherwise
  defp ensure_exported(module, function, arity) do
    case function_exported?(module, function, arity) do
      true -> :ok
      false -> {:error, {:module_does_not_export, {module, function, arity}}}
    end
  end

  defp default_implementation do
    module_config = Application.get_env(:ockam, __MODULE__, [])
    Keyword.get(module_config, :default, @default_implementation)
  end

  def format_error(%DecodeError{reason: :decoder_is_nil_and_no_default_decoder}),
    do: "Decoder argument is nil and there is no default decoder configured."

  def format_error(%DecodeError{reason: {:decoder_is_not_a_module, decoder}}),
    do: "Decoder argument is not a module: #{inspect(decoder)}"

  def format_error(%DecodeError{reason: {:encoded_input_is_not_binary, encoded}}),
    do: "Encoded input cannot be decoded as it is not a binary: #{inspect(encoded)}"

  def format_error(%DecodeError{reason: {:module_not_loaded, {:decoder, module}}}),
    do: "Decoder module is not loaded: #{inspect(module)}"

  def format_error(%DecodeError{reason: {:module_does_not_export, {module, :decode, 1}}}),
    do: "Decoder module does not export: #{inspect(module)}.decode/1"

  def format_error(%EncodeError{reason: :encoder_is_nil_and_no_default_encoder}),
    do: "Encoder argument is nil and there is no default encoder configured."

  def format_error(%EncodeError{reason: {:encoder_is_not_a_module, encoder}}),
    do: "Encoder argument is not a module: #{inspect(encoder)}"

  def format_error(%EncodeError{reason: {:module_not_loaded, {:encoder, module}}}),
    do: "Encoder module is not loaded: #{inspect(module)}"

  def format_error(%EncodeError{reason: {:module_does_not_export, {module, :encode, 1}}}),
    do: "Encoder module does not export: #{inspect(module)}.encode/1"

  def format_error(%DecodeError{reason: {:too_much_data, encoded, rest}}),
    do: "Too much data in #{inspect(encoded)} ; extra data: #{inspect(rest)}"

  def format_error(%DecodeError{reason: {:not_enough_data, data}}),
    do: "Not enough data in #{inspect(data)}"

  def format_error(%DecodeError{reason: {:invalid_version, data, version}}),
    do: "Unknown message format or version: #{inspect(version)} #{inspect(data)}"

  def format_error(%{reason: reason}), do: inspect(reason)
end
