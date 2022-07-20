defmodule Ockam.API.Request do
  @moduledoc """
  Ockam request-response API request
  """

  alias Ockam.API.Request
  defstruct [:id, :path, :method, :body, from_route: [], to_route: [], local_metadata: %{}]

  @max_id 65_534

  defmodule Header do
    @moduledoc """
    Request header, minicbor schema definition
    """
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:id, integer(), minicbor: [key: 1])
      field(:path, String.t(), minicbor: [key: 2])

      field(:method, :get | :post | :put | :delete | :patch,
        minicbor: [key: 3, schema: {:enum, [get: 0, post: 1, put: 2, delete: 3, patch: 4]}]
      )

      field(:has_body, boolean(), minicbor: [key: 4])
    end
  end

  def encode(request) when is_map(request) do
    body = request.body

    has_body =
      case body do
        nil -> false
        binary when is_binary(binary) -> true
        other -> raise "Body is not binary: #{inspect(other)}"
      end

    base =
      Header.encode(%Header{
        id: request.id,
        path: request.path,
        method: request.method,
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
            {:ok, %Request{id: header.id, method: header.method, path: header.path, body: body}}

          {false, false} ->
            {:ok, %Request{id: header.id, method: header.method, path: header.path, body: nil}}

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
        return_route: return_route,
        local_metadata: local_metadata
      }) do
    with {:ok, %__MODULE__{} = request} <- decode(payload) do
      {:ok,
       %{
         request
         | from_route: return_route,
           to_route: onward_route,
           local_metadata: local_metadata
       }}
    end
  end

  def to_message(
        %__MODULE__{to_route: to_route, local_metadata: local_metadata} = request,
        return_route
      ) do
    %Ockam.Message{
      payload: encode(request),
      onward_route: to_route,
      return_route: return_route,
      local_metadata: local_metadata
    }
  end

  @spec caller_identity(%__MODULE__{}) :: :error | {:ok, identity :: Ockam.Identity.t()}
  def caller_identity(%__MODULE__{local_metadata: meta}) do
    Map.fetch(meta, :identity)
  end

  @spec caller_identity_id(%__MODULE__{}) :: :error | {:ok, identity_id :: String.t()}
  def caller_identity_id(%__MODULE__{local_metadata: meta}) do
    Map.fetch(meta, :identity_id)
  end
end
