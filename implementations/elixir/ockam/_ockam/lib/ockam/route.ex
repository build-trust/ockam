defmodule Ockam.Route do
  @moduledoc """
  A route is an ordered list of addresses.
  """

  alias Ockam.Address

  @type t :: [Address.t()]
end
