defmodule Ockam.Messaging.ConfirmPipe.Wrapper do
  @moduledoc """
  Resend pipe message wrapper.

  Wraps message with a send reference (integer)
  """

  ## Message is encoded with ockam wire protocol
  ## and combined with the ref using the following BARE schema
  @bare_schema {:struct, [ref: :uint, data: :data]}

  def wrap_message(%Ockam.Message{} = message, ref) do
    case Ockam.Wire.encode(message) do
      {:ok, encoded_message} ->
        {:ok, :bare.encode(%{ref: ref, data: encoded_message}, @bare_schema)}

      error ->
        error
    end
  end

  def unwrap_message(wrapped) do
    with {:ok, %{ref: ref, data: encoded_message}, ""} <- :bare.decode(wrapped, @bare_schema),
         {:ok, message} <- Ockam.Wire.decode(encoded_message, :confirm_pipe) do
      {:ok, ref, message}
    else
      {:ok, _decoded, _rest} = bare_result ->
        {:error, {:bare_decode_error, wrapped, bare_result}}

      {:error, err} ->
        {:error, err}
    end
  end
end
