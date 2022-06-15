defmodule Ockam.API.Response do
  @moduledoc """
  Ockam request-response API response
  """

  alias Ockam.API.Request
  alias Ockam.API.Response

  defstruct [:id, :request_id, :status, :body, from_route: [], to_route: []]

  @max_id 65_534

  defmodule Header do
    @moduledoc """
    Response header, minicbor schema definition
    """
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:id, integer(), minicbor: [key: 1])
      field(:request_id, integer(), minicbor: [key: 2])
      field(:status, integer(), minicbor: [key: 3])
      field(:has_body, boolean(), minicbor: [key: 4])
    end
  end

  def encode(response) when is_map(response) do
    body = response.body

    has_body =
      case body do
        nil -> false
        binary when is_binary(binary) -> true
        other -> raise "Body is not binary: #{inspect(other)}"
      end

    base =
      Header.encode(%Header{
        id: response.id,
        request_id: response.request_id,
        status: response.status,
        has_body: has_body
      })

    payload =
      case has_body do
        true -> base <> body
        false -> base
      end

    :bare.encode(payload, :data)
  end

  def decode(data) when is_binary(data) do
    {:ok, payload, ""} = :bare.decode(data, :data)

    case Header.decode(payload) do
      {:ok, header, body} ->
        body_present = byte_size(body) > 0

        case {header.has_body, body_present} do
          {true, _} ->
            {:ok,
             %Response{
               id: header.id,
               request_id: header.request_id,
               status: header.status,
               body: body
             }}

          {false, false} ->
            {:ok,
             %Response{
               id: header.id,
               request_id: header.request_id,
               status: header.status,
               body: nil
             }}

          {false, true} ->
            {:error, {:decode_error, :unexpected_body, data}}
        end

      other ->
        {:error, {:decode_error, other, data}}
    end
  end

  def gen_id() do
    :rand.uniform(@max_id)
  end

  def reply_to(%Request{id: request_id, from_route: route}, status, body) do
    %__MODULE__{
      id: gen_id(),
      request_id: request_id,
      status: status,
      body: body,
      to_route: route
    }
  end

  def from_message(%Ockam.Message{
        payload: payload,
        onward_route: onward_route,
        return_route: return_route
      }) do
    with {:ok, %__MODULE__{} = request} <- decode(payload) do
      {:ok, %{request | from_route: return_route, to_route: onward_route}}
    end
  end

  def to_message(%__MODULE__{to_route: to_route} = response, return_route) do
    %Ockam.Message{
      payload: encode(response),
      onward_route: to_route,
      return_route: return_route
    }
  end
end
