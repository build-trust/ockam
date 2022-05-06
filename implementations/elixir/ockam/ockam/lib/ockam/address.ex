defmodule Ockam.Address do
  @moduledoc """
  Address data type, which cab be part of a route
  """
  defstruct [:type, :value]

  @typedoc "An integer between 0 and 255 that represents the type of an address."
  @type type :: 0..255 | atom

  @typedoc "Address can be a string or an %Ockam.Address{} structure"
  @type t() :: String.t() | %__MODULE__{type: type(), value: binary()}
  @type t(address_type) :: %__MODULE__{type: address_type, value: binary()}

  @type route() :: [t()]

  @doc """
  Returns `true` if `term` is a valid `t:Ockam.Router.Address.type/0`;
  otherwise returns `false`.

  Allowed in guard tests. Inlined by the compiler.
  """
  @doc guard: true
  Kernel.defguard(
    is_address_type(term)
    when is_integer(term) and term >= 0 and term <= 255
  )

  @doc """
  Returns the type of an address.
  """
  @spec type(t()) :: type()

  def type(string) when is_binary(string), do: 0
  def type(%{type: type}) when is_address_type(type), do: type

  @spec value(t()) :: binary()
  def value(string) when is_binary(string), do: string
  def value(%{value: value}) when is_binary(value), do: value

  def parse_route!(string_route) do
    ## TODO: handle errors
    string_route
    |> String.split(";")
    |> Enum.map(fn address ->
      [type, value] = String.split(address, "#", parts: 2)
      type = String.to_integer(type)
      %__MODULE__{type: type, value: value}
    end)
  end

  ## TODO: figure out a better API for address representation
  def normalize(address) do
    %{type: type(address), value: value(address)}
  end

  def denormalize(%{"type" => type, "value" => value}) do
    denormalize(%{type: type, value: value})
  end

  def denormalize(%{type: 0, value: string}), do: string
  def denormalize(string) when is_binary(string), do: string
  def denormalize(%{} = address), do: struct(__MODULE__, address)
end
