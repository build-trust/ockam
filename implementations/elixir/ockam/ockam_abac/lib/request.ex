defmodule Ockam.ABAC.Request do
  @moduledoc """
  ABAC.Request data structure

  Created by Policy Enforcement Point in order to be used in PolicyCheck
  """

  alias Ockam.ABAC.ActionId
  ## TODO: do we want to break action_id down to resource/action?
  @enforce_keys [:action_id, :action_attributes, :resource_attributes, :subject_attributes]
  defstruct [
    :action_id,
    action_attributes: %{},
    resource_attributes: %{},
    subject_attributes: %{}
  ]

  @type t() :: %__MODULE__{
          action_id: ActionId.t(),
          action_attributes: Map.t(),
          resource_attributes: Map.t(),
          subject_attributes: Map.t()
        }
end
