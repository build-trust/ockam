defmodule Ockam.Transport do
  @type opts :: Keyword.t()
  @type data :: iodata()
  @type reason :: term()
  @type t :: %{:__struct__ => module() | map()}

  @callback init(opts) :: t()
  @callback open(t()) :: {:ok, t()} | {:error, reason}
  @callback send(t(), data, opts) :: {:ok, t()} | {:error, reason}
  @callback recv(t(), opts) :: {:ok, data(), t()} | {:error, reason}
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
    size = byte_size(message)
    <<size::big-unsigned-size(2)-unit(8), message::binary>>
  end

  @doc """
  Decodes a raw data packet received from a transport connection
  """
  @spec decode(binary()) ::
          {:ok, binary(), binary()}
          | {:more, non_neg_integer()}
          | {:error, term}
  def decode(message) do
    :erlang.decode_packet(2, message, [])
  end
end
