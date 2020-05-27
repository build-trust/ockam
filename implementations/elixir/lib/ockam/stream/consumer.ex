defmodule Ockam.Stream.Consumer do
  @moduledoc """
  Defines an Ockam Stream Consumer
  """

  alias Ockam.Stream
  alias Ockam.Worker

  defstruct [:address]

  @typedoc "The stream consumer address type."
  @type address :: Router.address()

  @typedoc "The stream consumer type."
  @type t :: %__MODULE__{address: address}

  @doc """
  Creates a new stream consumer.
  """
  @spec create(Stream.t(), any, Keyword.t() | fun) :: {:ok, t} | {:error, term}
  def create(stream, handler_state \\ nil, handler_or_options) do
    with {:ok, worker} <- Worker.create(handler_state, handler_or_options),
         :ok <- Stream.attach_consumer(stream, worker.address) do
      {:ok, %__MODULE__{address: worker.address}}
    end
  end
end
