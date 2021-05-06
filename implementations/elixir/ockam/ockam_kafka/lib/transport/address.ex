defmodule Ockam.Kafka.Transport.Address do
  defstruct [:topic]

  def deserialize(data) do
    %__MODULE__{topic: data}
  end
end

defimpl Ockam.Address, for: Ockam.Kafka.Transport.Address do
  def type(_address), do: 3
  ## Should this be the topic?
  def value(address), do: address
end

defimpl Ockam.Serializable, for: Ockam.Kafka.Transport.Address do
  alias Ockam.Kafka.Transport.Address

  # address type
  @address_type 3

  def serialize(%Address{topic: topic}) do
    %{type: @address_type, value: topic}
  end
end
