defmodule Ockam.Services.Provider.Stream do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing stream services, :stream and :stream_index
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Stream.Index.Service, as: StreamIndexService
  alias Ockam.Stream.Workers.Service, as: StreamService

  @services [:stream, :stream_index]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:stream, args) do
    {StreamService, Keyword.merge([address: "stream"], args)}
  end

  def child_spec(:stream_index, args) do
    {StreamIndexService, Keyword.merge([address: "stream_index"], args)}
  end
end
