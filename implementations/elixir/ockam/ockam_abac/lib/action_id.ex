defmodule Ockam.ABAC.ActionId do
  @moduledoc """
  Data structure serving for association between ABAC.Request and ABAC.Policy
  In order to match, both Request and Policy should have the same ActionId

  Contains :resource and :action components.
  """
  use TypedStruct

  typedstruct do
    plugin(Ockam.TypedCBOR.Plugin)
    field(:resource, String.t(), minicbor: [key: 1])
    field(:action, String.t(), minicbor: [key: 2])
  end

  def new(resource, action) do
    %__MODULE__{resource: resource, action: action}
  end

  def parse(action_id_str) do
    case String.split(action_id_str, "/", parts: 2) do
      [resource, action] -> {:ok, __MODULE__.new(resource, action)}
      other -> {:error, {:invalid_action_id, other}}
    end
  end

  def format(%__MODULE__{resource: resource, action: action}) do
    "#{resource}/#{action}"
  end

  def match_action_id?(%__MODULE__{} = pattern, %__MODULE__{} = action) do
    match_resource?(pattern, action) and match_action?(pattern, action)
  end

  def match_resource?(%__MODULE__{resource: pattern}, %__MODULE__{resource: resource}) do
    String.match?(resource, Regex.compile!(pattern))
  end

  def match_action?(%__MODULE__{action: pattern}, %__MODULE__{action: action}) do
    String.match?(action, Regex.compile!(pattern))
  end
end
