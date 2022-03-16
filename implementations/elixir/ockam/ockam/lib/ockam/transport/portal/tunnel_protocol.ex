defmodule Ockam.Transport.Portal.TunnelProtocol do
  @moduledoc """
  Portal protocol messages
  """

  @typedoc """
  Tunneling protocol messages
  """
  alias Ockam.Bare.Extended, as: BareExt
  @type msg :: :ping | :pong | :disconnect | {:payload, binary()}

  @schema {:variant, [:ping, :pong, :disconnect, {:payload, :data}]}

  @spec encode(msg()) :: binary()
  def encode(t), do: BareExt.encode(t, @schema)

  @spec decode(binary()) :: {:ok, msg()} | {:error, any()}
  def decode(data), do: BareExt.decode(data, @schema)
end
