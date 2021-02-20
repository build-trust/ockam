defprotocol Ockam.Serializable do
  @moduledoc """
  Defines an elixir protocol for serializing a value to a binary.
  """

  @doc """
  Converts a value to iodata.

  Returns iodata, if it succeeds.
  Returns {:error, reason}, , if it fails.
  """
  @spec serialize(any) :: iodata | {:error, reason :: any}
  def serialize(value)
end

defimpl Ockam.Serializable, for: BitString do
  @moduledoc false

  def serialize(value) when is_binary(value) do
    %{type: 0, value: value}
  end

  def serialize(value) when is_bitstring(value), do: {:error, :value_is_a_bitstring}
end
