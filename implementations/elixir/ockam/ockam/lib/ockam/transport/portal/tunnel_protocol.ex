defmodule Ockam.Transport.Portal.TunnelProtocol do
  @moduledoc """
  Portal protocol messages
  """

  @typedoc """
  Tunneling protocol messages
  """
  alias Ockam.Bare.Extended, as: BareExt
  @type msg :: :ping | :pong | :disconnect | {:payload, {binary(), integer()}}

  @schema {:variant,
           [:ping, :pong, :disconnect, {:payload, {:tuple, [:data, {:optional, :u16}]}}]}

  @spec encode(msg()) :: binary()
  def encode({:payload, {data, counter}}) do
    BareExt.encode({:payload, [data, counter]}, @schema)
  end

  @spec encode(msg()) :: binary()
  def encode(data) do
    BareExt.encode(data, @schema)
  end

  @spec decode(binary()) :: {:ok, msg()} | {:error, any()}
  def decode(data) do
    with {:ok, {:payload, [data, packet_counter]}} <- BareExt.decode(data, @schema) do
      {:ok, {:payload, {data, packet_counter}}}
    end
  end
end
