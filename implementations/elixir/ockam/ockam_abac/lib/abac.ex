defmodule Ockam.ABAC do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Main application for Ockam ABAC
  """

  use Application

  require Logger

  @doc false
  def start(_type, _args) do
    children = policy_storage_child_specs()
    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  def default_policy_storage() do
    case Application.get_env(:ockam_abac, :policy_storage) do
      nil ->
        {:error, :policy_storage_not_configured}

      module when is_atom(module) ->
        {:ok, module}

      {module, _args} when is_atom(module) ->
        {:ok, module}
    end
  end

  defp policy_storage_child_specs() do
    case Application.get_env(:ockam_abac, :policy_storage) do
      nil ->
        []

      module when is_atom(module) ->
        [module]

      {module, args} when is_atom(module) and is_list(args) ->
        [{module, args}]
    end
  end
end
