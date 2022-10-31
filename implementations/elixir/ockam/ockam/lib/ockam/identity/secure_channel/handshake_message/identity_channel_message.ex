defmodule Ockam.Identity.SecureChannel.HandshakeMessage do
  @moduledoc """
  Data encoding/decoding for HandshakeMessage
  """

  alias Ockam.Identity.SecureChannel.HandshakeMessage.Request
  alias Ockam.Identity.SecureChannel.HandshakeMessage.Response

  @request {:struct, [contact: :data, proof: :data]}
  @response {:struct, [contact: :data, proof: :data]}

  def encode(%Request{} = request) do
    <<0>> <> :bare.encode(request, @request)
  end

  def encode(%Response{} = response) do
    <<1>> <> :bare.encode(response, @response)
  end

  def decode(<<type>> <> data) do
    case type do
      0 ->
        decode_request(data)

      1 ->
        decode_response(data)

      _other ->
        {:error, {:decode_error, {:unknown_type, type}, data}}
    end
  end

  def decode(data) do
    {:error, {:decode_error, :not_enough_data, data}}
  end

  def decode_request(data) do
    case :bare.decode(data, @request) do
      {:ok, map, ""} ->
        {:ok, struct(Request, map)}

      {:ok, _map, extra} ->
        {:error, {:decode_error, {:extra_data, extra}, data}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def decode_response(data) do
    case :bare.decode(data, @request) do
      {:ok, map, ""} ->
        {:ok, struct(Response, map)}

      {:ok, _map, extra} ->
        {:error, {:decode_error, {:extra_data, extra}, data}}

      {:error, reason} ->
        {:error, reason}
    end
  end
end
