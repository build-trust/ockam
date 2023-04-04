defmodule Ockam.Services.API.NodeInfo.Info do
  @moduledoc """
  Data structure to communicate node information
  Currently only contains version
  """
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:version, String.t(), minicbor: [key: 1])
  end
end

defmodule Ockam.Services.API.NodeInfo do
  @moduledoc """
  API worker to show node information
  Just returns the information provided on worker setup

  Options:
  - info: Ockam.Services.API.NodeInfo.Info - information to return
  """
  use Ockam.Services.API

  alias Ockam.API.Request
  alias Ockam.Services.API.NodeInfo.Info

  def node_info(worker, timeout \\ 5000) do
    Ockam.Worker.call(worker, :get, timeout)
  end

  @impl Ockam.Worker
  def setup(options, state) do
    case Keyword.fetch(options, :info) do
      {:ok, %Info{version: version} = info} when is_binary(version) ->
        {:ok, Map.put(state, :info, info)}

      {:ok, info} ->
        {:error, {:invalid_node_info, info}}

      :error ->
        {:error, {:option_required, :node_info}}
    end
  end

  @impl Ockam.Services.API
  def handle_request(%Request{method: :get, path: ""}, state) do
    response = encode_response(get_node_info(state))
    {:reply, :ok, response, state}
  end

  @impl GenServer
  def handle_call(:get, _from, state) do
    {:reply, get_node_info(state), state}
  end

  defp encode_response(info) do
    Info.encode!(info)
  end

  defp get_node_info(%{info: info}) do
    info
  end
end
