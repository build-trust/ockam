defmodule Ockam.Wire do
  @moduledoc """
  Encodes and decodes messages that can be transported on the wire.
  """

  alias Ockam.Message

  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  @doc """
  Encodes a message into a binary.

  Returns `{:ok, binary}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback encode(message :: Message.t()) ::
              {:ok, encoded :: binary()} | {:error, error :: EncodeError.t()}

  @doc """
  Decodes a message from a binary.

  Returns `{:ok, message}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @callback decode(encoded :: binary()) ::
              {:ok, message :: Message.t()} | {:error, error :: DecodeError.t()}

  @doc """
  Formats an error returned by `Ockam.Wire.encode/1` or `Ockam.Wire.decode/1`.

  Returns a string.
  """
  @callback format_error(error :: EncodeError.t() | DecodeError.t()) ::
              formatted_error_message :: String.t()

  # Set the default encoder/decoder from application config.
  @default_implementation Keyword.get(
                            Application.get_env(:ockam, __MODULE__, []),
                            :default_implementation
                          )

  @doc """
  Encode a message to a binary using the provided encoder.
  """
  @spec encode(encoder :: atom(), message :: Message.t()) ::
          {:ok, encoded :: binary()} | {:error, error :: EncodeError.t()}

  def encode(encoder \\ @default_implementation, message) do
    with :ok <- ensure_loaded(:encoder, encoder),
         :ok <- ensure_exported(:encoder, encoder, :encode, 1) do
      encoder.encode(message)
    else
      {:error, reason} -> {:error, %EncodeError{reason: reason, module: __MODULE__}}
    end
  end

  @doc """
  Decode a message from binary using the provided decoder.
  """
  @spec decode(decoder :: atom(), encoded :: binary()) ::
          {:ok, message :: Message.t()} | {:error, error :: DecodeError.t()}

  def decode(decoder \\ @default_implementation, encoded)

  def decode(decoder, encoded) when is_binary(encoded) do
    with :ok <- ensure_loaded(:decoder, decoder),
         :ok <- ensure_exported(:decoder, decoder, :decode, 1) do
      decoder.decode(encoded)
    else
      {:error, reason} -> {:error, %DecodeError{reason: reason, module: __MODULE__}}
    end
  end

  def decode(_decoder, encoded),
    do: {:error, %DecodeError{reason: {:argument_is_not_binary, encoded}}}

  # returns :ok if module is loaded, {:error, reason} otherwise
  defp ensure_loaded(type, module) do
    case Code.ensure_loaded?(module) do
      true -> :ok
      false -> {:error, {:not_loaded, type, module}}
    end
  end

  # returns :ok if a module exports the given function, {:error, reason} otherwise
  defp ensure_exported(type, module, function, arity) do
    case function_exported?(module, function, arity) do
      true -> :ok
      false -> {:error, {:not_exported, type, {module, function, arity}}}
    end
  end

  @doc false
  # use Exception.message(error) to get formatted error messages.
  def format_error(%EncodeError{reason: {:not_loaded, :encoder, nil}}),
    do: "Encoder module is nil."

  def format_error(%EncodeError{reason: {:not_loaded, :encoder, module}}),
    do: "Encoder module could not be loaded: #{inspect(module)}"

  def format_error(%EncodeError{reason: {:not_exported, :encoder, {module, :encode, 1}}}),
    do: "Encoder module (#{module}) does not export #{module}.encode/1"

  def format_error(%DecodeError{reason: {:not_loaded, :decoder, nil}}),
    do: "Decoder module is nil."

  def format_error(%DecodeError{reason: {:not_loaded, :decoder, module}}),
    do: "Decoder module could not be loaded: #{inspect(module)}"

  def format_error(%DecodeError{reason: {:not_exported, :decoder, {module, :decode, 1}}}),
    do: "Decoder module (#{module}) does not export #{module}.decode/1"
end
