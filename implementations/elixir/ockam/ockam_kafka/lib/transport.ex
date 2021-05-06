defmodule Ockam.Kafka.Transport do
  @moduledoc false

  alias Ockam.Kafka.Transport.Publisher
  alias Ockam.Node
  alias Ockam.Router

  @spec start(keyword) :: :ignore | {:error, any} | {:ok, any}
  @doc false
  def start(options) do
    ## Publisher starts a consumer
    with {:ok, _} = Ockam.Kafka.Transport.Consumer.start_link(options),
         {:ok, address} = Publisher.create(options) do
      setup_routed_message_handler(address)
    end
  end

  defp setup_routed_message_handler(address) do
    handler = fn message ->
      Node.send(address, message)
    end

    with :ok <- Router.set_message_handler(3, handler),
         :ok <- Router.set_message_handler(Ockam.Kafka.Transport.Address, handler) do
      :ok
    end
  end
end
