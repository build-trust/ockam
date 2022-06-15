defmodule Ockam.Messaging.IndexPipe.Wrapper do
  @moduledoc """
  Message wrapper for indexed pipes
  """

  ## Message is encoded with ockam wire protocol
  ## and combined with the index using the following BARE schema
  @schema {:struct, [index: :uint, message: :data]}

  @doc """
  Encodes message and index into a binary
  """
  @spec wrap_message(integer(), Ockam.Message.t()) :: binary()
  def wrap_message(index, %Ockam.Message{} = message) do
    {:ok, encoded} = Ockam.Wire.encode(message)
    :bare.encode(%{index: index, message: encoded}, @schema)
  end

  @doc """
  Decodes message and index from a binary
  """
  @spec unwrap_message(binary()) :: {:ok, integer(), Ockam.Message.t()} | {:error, any()}
  def unwrap_message(payload) do
    with {:ok, %{index: index, message: encoded_message}, ""} <-
           :bare.decode(payload, @schema),
         {:ok, message} <- Ockam.Wire.decode(encoded_message, :index_pipe) do
      {:ok, index, message}
    else
      {:ok, _decoded, _rest} = bare_result ->
        {:error, {:bare_decode_error, payload, bare_result}}

      {:error, err} ->
        {:error, err}
    end
  end
end
