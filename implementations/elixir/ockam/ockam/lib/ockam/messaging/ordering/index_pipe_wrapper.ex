defmodule Ockam.Messaging.Ordering.IndexPipe.Wrapper do
  @moduledoc """
  Message wrapper for indexed pipes
  """

  @schema {:struct, [index: :uint, message: :data]}

  @doc """
  Encodes message and index into a binary
  """
  @spec wrap_message(integer(), Ockam.Message.t()) :: binary()
  def wrap_message(index, message) do
    {:ok, encoded} = Ockam.Wire.encode(Ockam.Wire.Binary.V2, message)
    :bare.encode(%{index: index, message: encoded}, @schema)
  end

  @doc """
  Decodes message and index from a binary
  """
  @spec unwrap_message(binary()) :: {:ok, integer(), Ockam.Message.t()} | {:error, any()}
  def unwrap_message(payload) do
    with {:ok, %{index: index, message: encoded_message}, ""} <-
           :bare.decode(payload, @schema),
         {:ok, message} <- Ockam.Wire.decode(Ockam.Wire.Binary.V2, encoded_message) do
      {:ok, index, message}
    end
  end
end
