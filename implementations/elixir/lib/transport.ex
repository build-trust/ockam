defmodule Ockam.Transport do
  import Ockam.Router.Protocol.Encoding.Helpers

  alias Ockam.Router.Protocol.DecodeError

  @type opts :: Keyword.t()
  @type data :: iodata()
  @type reason :: term()
  @type t :: %{:__struct__ => module() | map()}

  @callback init(opts) :: t()
  @callback open(t()) :: {:ok, t()} | {:error, reason}
  @callback send(t(), data, opts) :: {:ok, t()} | {:error, reason}
  @callback recv(t(), opts) :: {:ok, data(), t()} | {:error, reason}
  @callback recv_nonblocking(t(), opts) ::
              {:ok, data(), t()} | {:wait, any(), t()} | {:error, reason}
  @callback close(t()) :: {:ok, t()} | {:error, reason()}

  @doc """
  Open a connection using the transport
  """
  def open(%callback_mod{} = transport) do
    callback_mod.open(transport)
  end

  @doc """
  Send data using the transport
  """
  def send(%callback_mod{} = transport, data, opts \\ []) do
    callback_mod.send(transport, data, opts)
  end

  @doc """
  Receive data using the transport
  """
  def recv(%callback_mod{} = transport, opts \\ []) do
    callback_mod.recv(transport, opts)
  end

  @doc """
  Receive data using the transport
  """
  def recv_nonblocking(%callback_mod{} = transport, opts \\ []) do
    callback_mod.recv_nonblocking(transport, opts)
  end

  @doc """
  Close the transport
  """
  def close(%callback_mod{} = transport) do
    callback_mod.close(transport)
  end

  @doc """
  Encodes a message for transmission over a transport connection
  """
  @spec encode(iodata() | binary()) :: binary()
  def encode(message)

  def encode(message) when is_list(message) do
    encode(IO.iodata_to_binary(message))
  end

  def encode(message) when is_binary(message) do
    size = encode_leb128_u2(byte_size(message))
    <<size::binary, message::binary>>
  end

  @doc """
  Decodes a raw data packet received from a transport connection
  """
  @spec decode(binary()) ::
          {:ok, binary(), binary()}
          | {:more, non_neg_integer()}
          | {:error, term}
  def decode(message) when is_binary(message) do
    {size, rest} = decode_leb128_u2(message)

    case rest do
      <<data::binary-size(size), extra::binary>> ->
        {:ok, data, extra}

      _ ->
        {:more, size - byte_size(rest)}
    end
  catch
    :throw, %DecodeError{} = err ->
      {:error, err}
  end
end
