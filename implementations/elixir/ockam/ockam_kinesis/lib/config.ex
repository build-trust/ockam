defmodule Ockam.Kinesis.Config do
  @moduledoc """
  Configuration helpers for AWS client
  """

  @spec access_key_id() :: String.t()
  def access_key_id() do
    Application.fetch_env!(:ockam_kinesis, :access_key_id)
  end

  @spec secret_access_key() :: String.t()
  def secret_access_key() do
    Application.fetch_env!(:ockam_kinesis, :secret_access_key)
  end

  @spec region() :: String.t()
  def region() do
    Application.fetch_env!(:ockam_kinesis, :region)
  end

  @spec http_client() :: {module(), Keyword.t()}
  def http_client() do
    Application.get_env(:ockam_kinesis, :http_client, {AWS.HTTPClient, []})
  end
end
