defmodule Ockam.API.Request do
  @moduledoc """
  Ockam request-response API request
  """

  defstruct [:id, :path, :method, :body, from_route: [], to_route: []]

  @max_id 65_534

  @method_schema {:enum, [:get, :post, :put, :delete, :patch]}
  @schema {:map, [:id, :path, {:method, @method_schema}, :has_body]}

  def encode(request) when is_map(request) do
    body = request.body

    has_body =
      case body do
        nil -> false
        binary when is_binary(binary) -> true
        other -> raise "Body is not binary: #{inspect(other)}"
      end

    request = Map.put(request, :has_body, has_body)

    base = MiniCBOR.encode(request, @schema)

    case has_body do
      true -> base <> body
      false -> base
    end
  end

  def decode(data) when is_binary(data) do
    case MiniCBOR.decode(data, @schema) do
      {:ok, decoded, body} ->
        has_body = Map.get(decoded, :has_body)
        body_present = byte_size(body) > 0

        case {has_body, body_present} do
          {true, _} ->
            {:ok, struct(__MODULE__, Map.put(decoded, :body, body))}

          {false, false} ->
            {:ok, struct(__MODULE__, Map.put(decoded, :body, nil))}

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

  def from_message(%Ockam.Message{
        payload: payload,
        onward_route: onward_route,
        return_route: return_route
      }) do
    with {:ok, %__MODULE__{} = request} <- decode(payload) do
      {:ok, %{request | from_route: return_route, to_route: onward_route}}
    end
  end

  def to_message(%__MODULE__{to_route: to_route} = request, return_route) do
    %Ockam.Message{
      payload: encode(request),
      onward_route: to_route,
      return_route: return_route
    }
  end
end
