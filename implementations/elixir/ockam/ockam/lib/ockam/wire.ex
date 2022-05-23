defmodule Ockam.Wire do
  @moduledoc """
  Encodes and decodes messages that can be transported on the wire.
  """

  alias Ockam.Message

  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  @doc """
  Encodes a message into a binary.

  Returns `{:ok, iodata}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback encode(message :: Message.t()) ::
              {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}

  @doc """
  Decodes a message from a binary.

  Returns `{:ok, message}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback decode(encoded :: binary()) ::
              {:ok, message :: Message.t()} | {:error, error :: DecodeError.t()}

  @doc """
  Encode a message to a binary using the provided encoder.
  """
  @spec encode(encoder :: atom, message :: Message.t()) ::
          {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}

  def encode(encoder \\ nil, message)

  def encode(nil, message) do
    case default_implementation() do
      nil -> {:error, EncodeError.new(:encoder_is_nil_and_no_default_encoder)}
      encoder -> encode(encoder, message)
    end
  end

  def encode(encoder, message) when is_atom(encoder) do
    with :ok <- ensure_loaded(:encoder, encoder),
         :ok <- ensure_exported(encoder, :encode, 1) do
      encoder.encode(message)
    else
      {:error, reason} -> {:error, EncodeError.new(reason)}
    end
  end

  def encode(encoder, _message) when not is_atom(encoder) do
    {:error, EncodeError.new({:encoder_is_not_a_module, encoder})}
  end

  @doc """
  Decode a message from binary using the provided decoder.
  """
  @spec decode(decoder :: atom, encoded :: binary) ::
          {:ok, message :: Message.t()} | {:error, error :: DecodeError.t()}

  def decode(decoder \\ nil, encoded)

  def decode(nil, encoded) when is_binary(encoded) do
    case default_implementation() do
      nil -> {:error, DecodeError.new(:decoder_is_nil_and_no_default_decoder)}
      decoder -> decode(decoder, encoded)
    end
  end

  def decode(decoder, encoded) when is_atom(decoder) and is_binary(encoded) do
    with :ok <- ensure_loaded(:decoder, decoder),
         :ok <- ensure_exported(decoder, :decode, 1),
         {:ok, message} <- decoder.decode(encoded) do
      {:ok, message}
    else
      {:error, reason} -> {:error, DecodeError.new(reason)}
    end
  end

  def decode(decoder, _encoded) when not is_atom(decoder) do
    {:error, DecodeError.new({:decoder_is_not_a_module, decoder})}
  end

  def decode(_decoder, encoded) when not is_binary(encoded) do
    {:error, DecodeError.new({:encoded_input_is_not_binary, encoded})}
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
    Keyword.get(module_config, :default)
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
