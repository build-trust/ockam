defmodule Ockam.Stream.Transport.Address do
  @moduledoc "Ockam address definition for stream transport"
  defstruct [:onward_stream, :return_stream]

  @schema {:struct, [onward_stream: :string, return_stream: :string]}

  def address_type(), do: 4

  def deserialize(data) do
    case :bare.decode(data, @schema) do
      {:ok, %{onward_stream: onward_stream, return_stream: return_stream}, ""} ->
        %__MODULE__{onward_stream: onward_stream, return_stream: return_stream}

      other ->
        raise("Stream transport deserialize error: #{inspect(other)}")
    end
  end

  def serialize(address) do
    :bare.encode(address, @schema)
  end
end

defimpl Ockam.Address, for: Ockam.Stream.Transport.Address do
  def type(_address), do: Ockam.Stream.Transport.Address.address_type()
  ## Should this be the onward_stream?
  def value(address), do: address
end

defimpl Ockam.Serializable, for: Ockam.Stream.Transport.Address do
  alias Ockam.Stream.Transport.Address

  def serialize(%Address{} = address) do
    %{type: Address.address_type(), value: Address.serialize(address)}
  end
end
