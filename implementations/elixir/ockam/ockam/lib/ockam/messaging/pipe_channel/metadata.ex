defmodule Ockam.Messaging.PipeChannel.Metadata do
  @moduledoc """
  Encodable data structure for pipechannel handshake metadata

  `receiver_route` - local route to receiver worker
  `channel_route` - local route to the channel worker (inner address)
  """

  defstruct [:receiver_route, :channel_route]

  @type t() :: %__MODULE__{}

  ## TODO: use proper address encoding
  @schema {:struct, [receiver_route: {:array, :data}, channel_route: {:array, :data}]}

  @spec encode(t()) :: binary()
  def encode(meta) do
    :bare.encode(meta, @schema)
  end

  @spec decode(binary()) :: t()
  def decode(data) do
    case :bare.decode(data, @schema) do
      {:ok, meta, ""} ->
        struct(__MODULE__, meta)

      other ->
        exit({:meta_decode_error, data, other})
    end
  end
end
