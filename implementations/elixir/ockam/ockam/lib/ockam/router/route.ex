defmodule Ockam.Router.Route do
  @moduledoc """
  A route is an ordered list of addresses.
  """

  alias Ockam.Router.Address

  @type t :: [Address.t()]

  @doc """
  Returns the address type of the first address in the route.
  """
  @spec first_address_type(t()) :: Address.type()

  def first_address_type([]), do: nil
  def first_address_type([first_address | _rest_of_the_route]), do: Address.type(first_address)
end
