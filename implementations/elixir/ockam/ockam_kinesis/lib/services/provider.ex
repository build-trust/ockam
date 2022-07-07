defmodule Ockam.Services.Kinesis.Provider do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing Kinesis stream service :stream_kinesis
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Stream.Workers.Service, as: StreamService

  @services [:stream_kinesis]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(service_name, args) do
    mod = service_mod(service_name)
    options = service_options(service_name, args)
    {mod, options}
  end

  defp service_mod(:stream_kinesis) do
    StreamService
  end

  defp service_options(:stream_kinesis, args) do
    address = make_address(args, "stream_kinesis")

    stream_options = [
      storage_mod: Ockam.Stream.Storage.Kinesis,
      storage_options: []
    ]

    [address: address, stream_options: stream_options]
  end

  defp make_address(args, default_address) do
    address_prefix = Keyword.get(args, :address_prefix, "")
    base_address = Keyword.get(args, :address, default_address)
    prefix_address(base_address, address_prefix)
  end

  defp prefix_address(base_address, "") do
    base_address
  end

  defp prefix_address(base_address, prefix) do
    prefix <> "_" <> base_address
  end
end
